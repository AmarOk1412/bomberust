/**
 * Copyright (c) 2019, Sébastien Blin <sebastien.blin@enconn.fr>
 * All rights reserved.
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * * Redistributions of source code must retain the above copyright
 *  notice, this list of conditions and the following disclaimer.
 * * Redistributions in binary form must reproduce the above copyright
 *  notice, this list of conditions and the following disclaimer in the
 *  documentation and/or other materials provided with the distribution.
 * * Neither the name of the University of California, Berkeley nor the
 *  names of its contributors may be used to endorse or promote products
 *  derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND ANY
 * EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED. IN NO EVENT SHALL THE REGENTS AND CONTRIBUTORS BE LIABLE FOR ANY
 * DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
 * (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 * LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
 * ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 * SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 **/
use super::Player;
use super::game::{Action, Game};
use super::super::gen::utils::Direction;

use crate::bomber::net::msg::*;

use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};

/**
 * Represents a room, manage a Game
 */
pub struct Room {
    capacity: u32,
    pub players: HashMap<u64, Player>,
    pub game: Option<Arc<Mutex<Game>>>,
    pub game_thread: Option<thread::JoinHandle<()>>,
}

impl Room {
    /**
     * Creates a Room
     * @return  The created Room
     */
    pub fn new() -> Room {
        Room {
            capacity: 2048,
            players: HashMap::new(),
            game: None,
            game_thread: None
        }
    }

    /**
     * Join the room
     * @param id    The player id
     * @return      If the operation is successful
     */
    pub fn join(&mut self, id: u64) -> bool {
        if self.capacity <= self.players.len() as u32 + 1 {
            return false;
        }
        if self.game.is_some() {
            return false;
        }
        self.players.insert(id, Player {});
        true
    }

    /**
     * Leave the room
     * @param id    The player id
     * @return      If the operation is successful
     */
    pub fn remove_player(&mut self, id: u64) -> bool {
        if !self.players.contains_key(&id) {
            warn!("Can't remove player from room because not found");
            return false;
        }
        self.players.remove(&id);
        self.players.len() == 0
    }

    /**
     * Launch the game
     * @param id    The player id who launch the game
     * @return      If the operation is successful
     */
    pub fn launch_game(&mut self, _id: u64) -> bool {
        if self.game.is_some() {
            warn!("Game already launched");
            return false;
        }
        let game = Arc::new(Mutex::new(Game::new()));
        let game_cloned = game.clone();
        self.game = Some(game);
        self.game_thread = Some(thread::spawn(move || {
            game_cloned.lock().unwrap().start();
            loop {
                game_cloned.lock().unwrap().event_loop();
                thread::sleep(Duration::from_nanos(1));
            }
        }));

        true
    }

    /**
     * Add a bomb
     * @param id    The player id
     * @return      If the operation is successful
     */
    pub fn put_bomb(&mut self, _id: u64) -> bool {
        if self.game.is_none() {
            warn!("No game launched, so cannot put bomb");
            return false;
        }
        self.game.as_ref().unwrap().lock().unwrap().push_action(Action::PutBomb, 0);

        true
    }

    /**
     * A player move in a direction
     * @param id        The player id
     * @param direction The direction chosen
     * @return          If the operation is successful
     */
    pub fn move_player(&mut self, _id: u64, direction: Direction) -> bool {
        if self.game.is_none() {
            warn!("No game launched, so cannot put bomb");
            return false;
        }
        self.game.as_ref().unwrap().lock().unwrap().push_action(Action::Move(direction), 0);

        true
    }

    pub fn get_map_msg(&self) -> MapMsg {
        MapMsg::new(self.game.as_ref().unwrap().lock().unwrap().map.clone())
    }
}