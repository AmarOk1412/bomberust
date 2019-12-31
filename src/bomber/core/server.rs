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
use super::super::gen::utils::Direction;
use super::Room;
use crate::serde::Serialize;

use rmps::Serializer;
use std::collections::HashMap;
use std::sync::{ Arc, Mutex };

use super::ia::NeuralNetwork;
use super::game::*;
use std::thread;
use std::time::Duration;
use rand::Rng;
use std::fs::File;
use std::io::prelude::*;
use std::fs;
use std::path::Path;

pub type PlayerStream = Arc<Mutex<Option<Vec<u8>>>>;
pub type GameStream = Arc<Mutex<Vec<Vec<u8>>>>;
struct Stream {
    pub tx: PlayerStream,
    pub rx: GameStream
}

/**
 * Represent the main server, manage rooms
 */
pub struct Server {
    lobby: Room,
    rooms: HashMap<u64, Room>,
    player_to_room: HashMap<u64, u64>,
    current_room_id: u64,
    player_to_stream: HashMap<u64, Stream>
}

impl Server {
    /**
     * Create a new Server
     */
    pub fn new() -> Server {
        Server {
            lobby: Room::new(),
            rooms: HashMap::new(),
            player_to_room: HashMap::new(),
            current_room_id: 0,
            player_to_stream: HashMap::new(),
        }
    }

    pub fn train() {
        let mut current_generation = Vec::<(i32, NeuralNetwork)>::with_capacity(100);
        println!("Generate population");
        let mut rng = rand::thread_rng();
        for i in 0..100 {
            let path = format!("ia_data/nn_{}.nn", i);
            if Path::new(&*path).exists() {
                println!("Using saved neural network for nn {}", i);
                let nn : NeuralNetwork = serde_json::from_str(&*fs::read_to_string(&*path).unwrap()).unwrap();
                current_generation.push((0, nn));
            } else {
                let nn = NeuralNetwork::new(vec![7*4 + 3*13*11,/*732,64*/500,7]);
                current_generation.push((0, nn));
            }
        }
        let mut generation = 0;
        let mut game = Game::new();
        if Path::new("ia_data/game").exists() {
            game.map = serde_json::from_str(&*fs::read_to_string("ia_data/game").unwrap()).unwrap();
        } else {
            let serialized_map = serde_json::to_string(&game.map).unwrap();
            let _ = fs::create_dir("ia_data");
            let _ = fs::write("ia_data/game", &*serialized_map);
        }
        loop {
            generation += 1;
            let mut max_score = 0;
            let mut max_score_idx = 0;
            for g in 0..25 {
                println!("Generation: {} - Game: {}", generation, g);
                let mut game_cloned = game.clone();
                let players = current_generation[(g*4)..(g*4+4)].to_vec();
                game_cloned.nns = players;
                game_cloned.start();
                while !game_cloned.finished() {
                    game_cloned.event_loop();
                    thread::sleep(Duration::from_nanos(1));
                }
                for j in 0..4 {
                    current_generation[(g*4)+j].0 = game_cloned.scores[j];
                    println!("New score: {}", game_cloned.scores[j]);
                    if max_score < game_cloned.scores[j] {
                        max_score = game_cloned.scores[j];
                        max_score_idx = g;
                    }
                }
            }

            // Save current state
            if generation % 10 == 1 {
                println!("Save current state");
                for i in 0..4 {
                    let serialized_nn = serde_json::to_string(&current_generation[(max_score_idx*4)+i].1).unwrap();
                    let path = format!("ia_data/{}_best_game_{}.nn", generation, i);
                    let _ = fs::write(&*path, &*serialized_nn);
                    let path = format!("ia_data/best_game_{}.nn", i);
                    let _ = fs::write(&*path, &*serialized_nn);
                }
            }

            current_generation.sort_by(|a,b| b.0.cmp(&a.0));
            println!("Best score: {}", current_generation[0].0);

            if generation % 10 == 1 {
                for i in 0..100 {
                    let path = format!("ia_data/nn_{}.nn", i);
                    let serialized_nn = serde_json::to_string(&current_generation[i].1).unwrap();
                    let _ = fs::write(&*path, &*serialized_nn);
                }
            }

            // Generating the new generation
            println!("Generate new population");
            let mut new_generation = current_generation[0..10].to_vec();
            for n in 0..10 {
                for n2 in 0..9 {
                    let nn = current_generation[n2].1.clone();
                    new_generation.push((0, current_generation[n].1.cross(&nn)));
                }
            }

            println!("Mutate population");
            for i in 0..100 {
                if rng.gen_range(0, 100) <= 5 {
                    new_generation[i].1.mutate();
                }
            }

            current_generation = new_generation;
        }
    }

    /**
     * A new player is coming. Add it to the lobby
     * @param id    The player id
     * @return      If the operation is successful
     */
    pub fn join_server(&mut self, id: u64, player_stream: PlayerStream) -> bool {
        info!("Client ({}) is in the lobby", id);
        let rx = Arc::new(Mutex::new(Vec::new()));
        self.player_to_room.insert(id, 0);
        self.player_to_stream.insert(id, Stream {
            tx: player_stream,
            rx: rx.clone(),
        });
        self.lobby.join(id, rx)
    }

    /**
     * A player is creating a room. Add it to this room at the end
     * @param id    The player id
     * @return      The id of the room created
     */
    pub fn create_room(&mut self, id: u64) -> u64 {
        if !self.player_to_room.contains_key(&id) {
            warn!("Can't create room because player is not in the server");
            return 0;
        }

        let room_id = self.player_to_room[&id];

        if room_id != 0 && !self.rooms.contains_key(&room_id) {
            warn!("Can't remove player from Room because rooms doesn't exists");
            return 0;
        }

        if room_id == 0 {
            self.lobby.remove_player(id);
        } else {
            let remove = self.rooms.get_mut(&room_id).unwrap().remove_player(id);
            if remove {
                info!("Remove room ({})", room_id);
                self.rooms.remove(&room_id);
            }
        }

        let mut room = Room::new_with_capacity(4);
        let rx = self.player_to_stream[&id].rx.clone();
        if room.join(id, rx) {
            self.current_room_id += 1;
            self.rooms.insert(self.current_room_id, room);
            *self.player_to_room.get_mut(&id).unwrap() = self.current_room_id;
            info!("Client ({}) is now in Room ({})", id, self.current_room_id);
        } else {
            *self.player_to_room.get_mut(&id).unwrap() = 0;
            warn!("Client ({}) can't join room. Going to room ({})", id, 0);
        }

        self.current_room_id
    }

    /**
     * A player is joining an existing room.
     * @param id        The player id
     * @param join_id   The room to join
     * @return          If the operation is successful
     */
    pub fn join_room(&mut self, id: u64, join_id: u64) -> bool {
        if !self.player_to_room.contains_key(&id) {
            warn!("Can't join room because player is not in the server");
            return false;
        }

        let room_id = self.player_to_room[&id];

        if room_id == join_id {
            warn!("Player try to join its current room");
            return false;
        }

        if room_id != 0 && !self.rooms.contains_key(&room_id) {
            warn!("Can't remove player from Room because rooms doesn't exists");
            return false;
        }

        if !self.rooms.contains_key(&join_id) {
            warn!("Player try to join inexistant room {}", join_id);
            return false;
        }

        if room_id == 0 {
            self.lobby.remove_player(id);
        } else {
            let remove = self.rooms.get_mut(&room_id).unwrap().remove_player(id);
            if remove {
                info!("Remove room ({})", room_id);
                self.rooms.remove(&room_id);
            }
        }

        let rx = self.player_to_stream[&id].rx.clone();
        if join_id == 0 {
            self.lobby.join(id, rx);
            *self.player_to_room.get_mut(&id).unwrap() = join_id;
        } else {
            if self.rooms.get_mut(&join_id).unwrap().join(id, rx) {
                *self.player_to_room.get_mut(&id).unwrap() = join_id;
                info!("Client ({}) is now in Room ({})", id, join_id);
            } else {
                *self.player_to_room.get_mut(&id).unwrap() = 0;
                warn!("Client ({}) can't join room. Going to room ({})", id, 0);
            }
        }

        true
    }

    pub fn leave_room(&mut self, id: u64) -> bool {
        if !self.player_to_room.contains_key(&id) {
            warn!("Can't leave room because player is not in the server");
            return false;
        }

        let room_id = self.player_to_room[&id];

        if room_id == 0 {
            warn!("Player try to leave lobby");
            return false;
        }

        if room_id != 0 && !self.rooms.contains_key(&room_id) {
            warn!("Can't remove player from Room because rooms doesn't exists");
            return false;
        }

        if room_id != 0 {
            let remove = self.rooms.get_mut(&room_id).unwrap().remove_player(id);
            if remove {
                info!("Remove room ({})", room_id);
                self.rooms.remove(&room_id);
            }
        }

        let rx = self.player_to_stream[&id].rx.clone();
        self.lobby.join(id, rx);
        *self.player_to_room.get_mut(&id).unwrap() = 0;
        info!("Client ({}) is now in Room ({})", id, 0);
        true
    }

    /**
     * A player is launching the game.
     * @param id        The player id
     * @return          If the operation is successful
     */
    pub fn launch_game(&mut self, id: u64) -> bool {
        if !self.player_to_room.contains_key(&id) {
            warn!("Can't launch game because player is not in the server");
            return false;
        }

        let room_id = self.player_to_room[&id];

        if room_id == 0 {
            warn!("Can't launch game from lobby");
            return false;
        }

        if !self.rooms.contains_key(&room_id) {
            warn!("Can't launch game because room doesn't exists");
            return false;
        }

        let mut game = Game::new();
        if Path::new("ia_data/game").exists() {
            game.map = serde_json::from_str(&*fs::read_to_string("ia_data/game").unwrap()).unwrap();
            let mut nns = Vec::<(i32, NeuralNetwork)>::with_capacity(4);
            for i in 0..4 {
                let path = format!("ia_data/best_game_{}.nn", i);
                if Path::new(&*path).exists() {
                    println!("Using saved neural network");
                    let nn : NeuralNetwork = serde_json::from_str(&*fs::read_to_string(&*path).unwrap()).unwrap();
                    nns.push((0, nn));
                } else {
                    let nn = NeuralNetwork::new(vec![7*4 + 3*13*11,/*732,64*/500,7]);
                    nns.push((0, nn));
                }
            }
            game.nns = nns;
        }
        self.rooms.get_mut(&room_id).unwrap().launch_game(id, Some(game));

        info!("Client ({}) launched game in room ({})", id, self.current_room_id);

        self.send_resources(room_id);

        true
    }

    /**
     * A player put a bomb.
     * @param id        The player id
     * @return          If the operation is successful
     */
    pub fn put_bomb(&mut self, id: u64) -> bool {
        if !self.player_to_room.contains_key(&id) {
            warn!("Can't put bomb because player is not in the server");
            return false;
        }

        let room_id = self.player_to_room[&id];

        if room_id == 0 {
            warn!("Can't put bomb from lobby");
            return false;
        }

        if !self.rooms.contains_key(&room_id) {
            warn!("Can't put bomb because room doesn't exists");
            return false;
        }

        self.rooms.get_mut(&room_id).unwrap().put_bomb(id);

        info!("Client ({}) putted bomb in room ({})", id, self.current_room_id);

        true
    }

    /**
     * A player move in a direction
     * @param id        The player id
     * @param direction The direction chosen
     * @return          If the operation is successful
     */
    pub fn move_player(&mut self, id: u64, direction: Direction) -> bool {
        if !self.player_to_room.contains_key(&id) {
            warn!("Can't move because player is not in the server");
            return false;
        }

        let room_id = self.player_to_room[&id];

        if room_id == 0 {
            warn!("Can't move from lobby");
            return false;
        }

        if !self.rooms.contains_key(&room_id) {
            warn!("Can't move because room doesn't exists");
            return false;
        }

        if self.rooms.get_mut(&room_id).unwrap().move_player(id, direction) {
            info!("Client ({}) moved {:?} in room ({})", id, direction, self.current_room_id);
        }

        true
    }

    pub fn get_events(&mut self, player: &u64) -> Vec<Vec<u8>> {
        if self.player_to_stream.contains_key(player) {
            let mut rx = self.player_to_stream[player].rx.lock().unwrap();
            let result = rx.clone();
            *rx = Vec::new();
            return result;
        }
        Vec::new()
    }

    fn send_resources(&mut self, room_id: u64) {
        info!("Sending resources for room {}", room_id);
        let players = self.rooms.get(&room_id).unwrap().players.keys();
        let mut buf = Vec::new();
        let msg = self.rooms.get(&room_id).unwrap().get_map_msg();
        msg.serialize(&mut Serializer::new(&mut buf)).unwrap();
        let len = buf.len() as u16;
        let mut send_buf : Vec<u8> = Vec::with_capacity(65536);
        send_buf.push((len >> 8) as u8);
        send_buf.push((len as u16 % (2 as u16).pow(8)) as u8);
        send_buf.append(&mut buf);

        for player in players {
            if self.player_to_stream.contains_key(player) {
                // TODO is it quick enough? Or add queue
                info!("Sending resources for player {}", player);
                *self.player_to_stream[player].tx.lock().unwrap() = Some(send_buf.clone());
            }
        }
    }
}