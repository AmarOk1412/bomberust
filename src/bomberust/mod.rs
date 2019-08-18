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

use std::time::{Duration, Instant};

pub mod map;

use map::{Map, Shape, BombItem};

#[derive(Clone)]
pub struct Player {
    id: i32,
    actions: Vec<Action>
}

#[derive(Clone)]
pub struct Bomb {
    pub radius: usize,
    pub shape: Shape,
    pub created_time: Instant,
    pub duration: Duration,
    pub pos: (usize, usize)
}

pub struct Game {
    pub map: Map,
    pub players: Vec<Player>,
    pub bombs: Vec<Bomb>,
    started: Instant,
}

#[derive(Clone)]
pub enum Action {
    PutBomb,
}

impl Game {
    pub fn new() -> Game {
        let map = Map::new(13, 11);
        print!("{}", map);
        let mut players = Vec::new();
        for id in 0..4 {
            players.push(Player {
                id,
                actions: Vec::new()
            });
        }
        Game {
            map,
            players,
            bombs: Vec::new(),
            started: Instant::now(),
        }
    }

    fn execute(&mut self, action: Action, player_id: i32) {
        match action {
            Action::PutBomb => {
                let player = &self.map.players[player_id as usize];
                if self.map.items[player.x + player.y * self.map.w].is_some() {
                    println!("CANNOT START BOMB!");
                    return;
                }
                self.map.items[player.x + player.y * self.map.w] = Some(Box::new(BombItem {}));
                self.bombs.push(Bomb {
                    radius: 1,
                    shape: Shape::Cross,
                    created_time: Instant::now(),
                    duration: Duration::new(3, 0),
                    pos: (player.x, player.y)
                });
            },
        }   
    }

    pub fn start(&mut self) {
        self.started = Instant::now();
        let mut printed = Instant::now();
        self.players[0].actions.push(Action::PutBomb);
        loop {
            let mut action_queue = Vec::new();
            for p in &mut self.players {
                match p.actions.pop() {
                    Some(a) => action_queue.push((a, p.id)),
                    _ => {}
                };
            }
            for (action, pid) in action_queue {
                self.execute(action, pid);
            }

            // Explode bomb
            for bomb in &self.bombs {
                if Instant::now() - bomb.duration >= bomb.created_time {
                    // Explode
                    self.map.items[bomb.pos.0 + self.map.w * bomb.pos.1] = None;
                    // Destroy items in zone
                    // TODO in several ticks
                    // Destroy players
                }
            }
            
            // print map
            if Instant::now() - printed > Duration::new(1,0) {
                printed = Instant::now();
                println!("New turn");
                println!("{}", self.map);
            }
        }
    }
}