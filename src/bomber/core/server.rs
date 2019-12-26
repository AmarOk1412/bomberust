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

        self.rooms.get_mut(&room_id).unwrap().launch_game(id);

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