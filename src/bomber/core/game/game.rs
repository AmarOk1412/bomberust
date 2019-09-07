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

use rand::Rng;
use std::cmp::{max, min};
use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};

use super::super::super::gen::{Map, item::*, utils::Shape};

// TODO redo this file

#[derive(Clone)]
pub struct Player {
    id: i32,
    actions: Vec<Action>
}

#[derive(Clone)]
pub struct ExplodingInfo {
    radius: i32,
    exploding_time: Instant,
    blocked_pos: HashSet<(i32, i32)>
}

#[derive(Clone)]
pub struct Bomb {
    pub radius: usize,
    pub shape: Shape,
    pub created_time: Instant,
    pub duration: Duration,
    pub pos: (usize, usize),
    pub exploding_info: Option<ExplodingInfo>,
}

pub struct Game {
    pub map: Map,
    pub players: Vec<Player>,
    pub bombs: Vec<Bomb>,
    started: Instant,
    last_printed: Instant,
    fps_instants: VecDeque<Instant>
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
            last_printed: Instant::now(),
            fps_instants: VecDeque::new(),
        }
    }

    pub fn start(&mut self) {
        self.started = Instant::now();
        self.last_printed = Instant::now();
    }


    pub fn push_action(&mut self, action: Action, player_id: u64) {
        self.players[player_id as usize].actions.push(action);
    }

    fn execute(&mut self, action: Action, player_id: i32) {
        match action {
            Action::PutBomb => {
                let player = &self.map.players[player_id as usize];
                if self.map.items[player.x + player.y * self.map.w].is_some() {
                    println!("CANNOT START BOMB!"); // TODO change with log
                    return;
                }
                self.map.items[player.x + player.y * self.map.w] = Some(Box::new(BombItem {}));
                self.bombs.push(Bomb {
                    radius: 2,
                    shape: Shape::Cross,
                    created_time: Instant::now(),
                    duration: Duration::from_secs(3),
                    pos: (player.x, player.y),
                    exploding_info: None,
                });
            },
        }   
    }

    fn execute_actions(&mut self) {
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
    }

    fn print_map(&mut self) {
        let now = Instant::now();
        let a_second_ago = now - Duration::from_secs(1);
        while self.fps_instants.front().map_or(false, |t| *t < a_second_ago) {
            self.fps_instants.pop_front();
        }
        self.fps_instants.push_back(now);
        let fps = self.fps_instants.len();

        if Instant::now() - self.last_printed > Duration::from_secs(1) {
            self.last_printed = Instant::now();
            println!("fps: {}\n{}", fps, self.map);
        }
    }

    pub fn event_loop(&mut self) {
        self.execute_actions();
        // Explode bomb
        // TODO clean
        let mut bomb_idx = 0;
        loop {
            if bomb_idx >= self.bombs.len() {
                break;
            }
            let mut bomb = &mut self.bombs[bomb_idx];
            if Instant::now() - bomb.duration >= bomb.created_time {
                // Explode
                let mut exploding_radius = 0;
                if bomb.exploding_info.is_none() {
                    bomb.exploding_info = Some(ExplodingInfo {
                        radius: 0,
                        exploding_time: Instant::now(),
                        blocked_pos: HashSet::new(),
                    });
                } else {
                    exploding_radius = bomb.exploding_info.as_ref().unwrap().radius;
                    if exploding_radius == bomb.radius as i32 {
                        // End of the bomb
                        self.map.items[bomb.pos.0 + self.map.w * bomb.pos.1] = None;
                        self.bombs.remove(bomb_idx);
                        continue;
                    }

                    let exploding_time = bomb.exploding_info.as_ref().unwrap().exploding_time;
                    //println!("{:?} {:?}", exploding_time, Instant::now());
                    if Instant::now() - Duration::from_millis((exploding_radius as u64 + 1) * 100) >= exploding_time {
                        exploding_radius += 1;
                        match bomb.exploding_info {
                            Some(ref mut info) => {
                                info.radius = exploding_radius;
                            }
                            _ => {}
                        };
                    }
                }

                for r in 0..(exploding_radius + 1) {
                    for x in (-r)..(r+1) {
                        for y in (-r)..(r+1) {
                            let pos = (bomb.pos.0 as i32 + x, bomb.pos.1 as i32 + y);
                            if pos.0 < 0 || pos.0 >= self.map.w as i32 {
                                continue;
                            }
                            if pos.1 < 0 || pos.1 >= self.map.h as i32 {
                                continue;
                            }
                            // TODO other than Cross
                            if pos.1 != bomb.pos.1 as i32 && pos.0 != bomb.pos.0 as i32 {
                                continue;
                            }
                            let (block, clear) = self.map.squares[pos.0 as usize + self.map.w * pos.1 as usize].sq_type.explode_event(&(pos.0 as usize, pos.1 as usize), &bomb.pos);
                            if block {
                                bomb.exploding_info.as_mut().unwrap().blocked_pos.insert(pos);
                            }
                            if clear {
                                // Test if any block in the path to the bomb
                                let mut clear = clear;
                                if pos.0 != bomb.pos.0 as i32 {
                                    let rev = pos.0 < bomb.pos.0 as i32;
                                    let min =  min(pos.0, bomb.pos.0 as i32);
                                    let max =  max(pos.0, bomb.pos.0 as i32);
                                    let range: Box<dyn Iterator<Item = i32>> = if rev {Box::new((min..max).rev())} else { Box::new(min..max) };

                                    for x in range {
                                        if bomb.exploding_info.as_ref().unwrap().blocked_pos.contains(&(x, pos.1)) {
                                            clear = false;
                                            break;
                                        }
                                    }
                                } else if pos.1 != bomb.pos.1 as i32 {
                                    let rev = pos.1 < bomb.pos.1 as i32;
                                    let min =  min(pos.1, bomb.pos.1 as i32);
                                    let max =  max(pos.1, bomb.pos.1 as i32);
                                    let range: Box<dyn Iterator<Item = i32>> = if rev {Box::new((min..max).rev())} else { Box::new(min..max) };

                                    for y in range {
                                        if bomb.exploding_info.as_ref().unwrap().blocked_pos.contains(&(pos.0, y)) {
                                            clear = false;
                                            break;
                                        }
                                    }
                                }
                                // TODO improve with raycasting tacting (angle + get distance to the next case touched)
                                if clear {
                                    // TODO if bomb, activate
                                    // Destroy items in zone
                                    let item = &self.map.items[pos.0 as usize + self.map.w * pos.1 as usize];
                                    if item.is_some() {
                                        // TODO as_ref for moving blocked_pos
                                        let (block, _) = item.as_ref().unwrap().explode_event(&(pos.0 as usize, pos.1 as usize), &bomb.pos);
                                        if block {
                                            bomb.exploding_info.as_mut().unwrap().blocked_pos.insert(pos);
                                        }

                                        let db = self.map.items[pos.0 as usize + self.map.w * pos.1 as usize].as_ref().unwrap().as_any().downcast_ref::<DestructibleBox>();
                                        match db {
                                            Some(_) => {
                                                let mut rng = rand::thread_rng();
                                                let prob = rng.gen_range(0, 5);
                                                if prob == 1 || prob == 2 {
                                                    let bonus : Bonus = rand::random();
                                                    self.map.items[pos.0 as usize + self.map.w * pos.1 as usize] = Some(Box::new(bonus));
                                                } else if prob == 3 {
                                                    let malus : Malus = rand::random();
                                                    self.map.items[pos.0 as usize + self.map.w * pos.1 as usize] = Some(Box::new(malus));
                                                } else {
                                                    self.map.items[pos.0 as usize + self.map.w * pos.1 as usize] = None;
                                                }
                                            },
                                            _ => {}
                                        }
                                    }
                                    // Destroy players
                                    let mut p = 0;
                                    loop {
                                        if p == self.map.players.len() {
                                            break;
                                        }
                                        let player = &self.map.players[p];
                                        if player.x == pos.0 as usize && player.y == pos.1 as usize {
                                            self.map.players.remove(p);
                                        } else {
                                            p += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            bomb_idx += 1;
        };

        self.print_map();
    }
}