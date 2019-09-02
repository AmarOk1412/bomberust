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
use super::Room;

use std::collections::HashMap;

pub struct Server {
    lobby: Room,
    rooms: HashMap<u64, Room>,
    player_to_room: HashMap<u64, u64>,
    current_room_id: u64,
}

impl Server {
    pub fn new() -> Server {
        Server {
            lobby: Room::new(),
            rooms: HashMap::new(),
            player_to_room: HashMap::new(),
            current_room_id: 0
        }
    }

    pub fn join_server(&mut self, id: u64) -> bool {
        info!("Client ({}) is in the lobby", id);
        self.player_to_room.insert(id, 0);
        self.lobby.join(id)
    }

    pub fn create_room(&mut self, id: u64) -> bool {
        if !self.player_to_room.contains_key(&id) {
            warn!("Can't create room because player is not in the server");
            return false;
        }

        let room_id = self.player_to_room[&id];

        if room_id != 0 && !self.rooms.contains_key(&room_id) {
            warn!("Can't remove player from Room because rooms doesn't exists");
            return false;
        }

        if room_id == 0 {
            self.lobby.remove_player(id);
        } else {
            let remove = self.rooms.get_mut(&room_id).unwrap().remove_player(id);
            if remove {
                self.rooms.remove(&room_id);
            }
        }

        let mut room = Room::new();
        if room.join(id) {
            self.current_room_id += 1;
            self.rooms.insert(self.current_room_id, room);
            *self.player_to_room.get_mut(&id).unwrap() = self.current_room_id;
            info!("Client ({}) is now in Room ({})", id, self.current_room_id);
        } else {
            *self.player_to_room.get_mut(&id).unwrap() = 0;
            info!("Client ({}) is now in Room ({})", id, 0);
        }
        
        true
    }

    pub fn join_room(&mut self, id: u64, join_id: u64) -> bool {
        if !self.player_to_room.contains_key(&id) {
            warn!("Can't create room because player is not in the server");
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

        if room_id == 0 {
            self.lobby.remove_player(id);
        } else {
            let remove = self.rooms.get_mut(&room_id).unwrap().remove_player(id);
            if remove {
                self.rooms.remove(&room_id);
            }
        }

        if join_id == 0 {
            self.lobby.join(id);
            *self.player_to_room.get_mut(&id).unwrap() = join_id;
        } else {
            if self.rooms.get_mut(&join_id).unwrap().join(id) {
                *self.player_to_room.get_mut(&id).unwrap() = join_id;
                info!("Client ({}) is now in Room ({})", id, join_id);
            } else {
                *self.player_to_room.get_mut(&id).unwrap() = 0;
                info!("Client ({}) is now in Room ({})", id, 0);
            }
        }

        true
    }

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
        
        true
    }

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
}