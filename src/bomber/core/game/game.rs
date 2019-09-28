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
use std::collections::{ HashMap, HashSet, VecDeque };
use std::time::{Duration, Instant};

use crate::bomber::core::Player;
use crate::bomber::gen::{Map, item::*, utils::*};
use crate::bomber::net::diff_msg::*;

// TODO redo this file

#[derive(Clone)]
pub struct GamePlayer {
    id: i32,
    actions: Vec<Action>,
    effects: Vec<PlayerEffect>
}

#[derive(Clone)]
pub struct ExplodingInfo {
    radius: i32,
    exploding_time: Instant,
    blocked_pos: HashSet<(i32, i32)>
}

#[derive(Clone)]
pub struct Bomb {
    pub creator_id: u32,
    pub radius: usize,
    pub shape: Shape,
    pub created_time: Instant,
    pub duration: Duration,
    pub pos: (usize, usize),
    pub exploding_info: Option<ExplodingInfo>,
}

pub struct Game {
    pub map: Map,
    pub players: Vec<GamePlayer>,
    pub bombs: Vec<Bomb>,
    pub game_player_to_player: HashMap<u64, Player>,
    started: Instant,
    last_printed: Instant,
    fps_instants: VecDeque<Instant>
}

#[derive(Clone)]
pub enum Action {
    PutBomb,
    Move(Direction),
}

impl Game {
    pub fn new() -> Game {
        let map = Map::new(13, 11);
        let mut players = Vec::new();
        for id in 0..4 {
            players.push(GamePlayer {
                id,
                actions: Vec::new(),
                effects: Vec::new()
            });
        }
        Game {
            map,
            players,
            bombs: Vec::new(),
            game_player_to_player: HashMap::new(),
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

    pub fn link_player(&mut self, player: Player) -> Option<u64> {
        if self.game_player_to_player.len() == self.players.len() {
            warn!("Can't link player because room is full");
            return None;
        }
        let id = self.game_player_to_player.len() as u64;
        self.game_player_to_player.insert(id, player);
        Some(id)
    }

    fn inform_players(&mut self, diff: &Vec<u8>) {
        for (_, player) in &mut self.game_player_to_player {
            player.rx.lock().unwrap().push(diff.clone());
        }
    }

    fn execute(&mut self, action: Action, player_id: i32) {
        match action {
            Action::PutBomb => {
                let player = &self.map.players[player_id as usize];
                if self.map.items[player.x as usize + player.y as usize * self.map.w].is_some() {
                    info!("Player {} Cannot start bomb with an item", player_id);
                    return;
                }
                let mut current_bombs = 0;
                for bomb in &self.bombs {
                    if bomb.creator_id == player_id as u32 {
                        current_bombs += 1;
                    }
                }
                if current_bombs >= player.bomb {
                    info!("Player {} already launch all the bomb", player_id);
                    return;
                }
                self.map.items[player.x as usize + player.y as usize * self.map.w] = Some(Box::new(BombItem {}));
                self.bombs.push(Bomb {
                    creator_id: player_id as u32,
                    radius: player.radius as usize,
                    shape: Shape::Cross,
                    created_time: Instant::now(),
                    duration: Duration::from_secs(3),
                    pos: (player.x as usize, player.y as usize),
                    exploding_info: None,
                });
                let diff = PlayerPutBomb {
                    msg_type: String::from("player_put_bomb_diff"),
                    id: player_id,
                    x: player.x as usize,
                    y: player.y as usize,
                };
                self.inform_players(&diff.to_vec());
            },
            Action::Move(direction) => {
                // TODO change increment (NOTE: if bomb under player, should be able to move)
                let player = &mut self.map.players[player_id as usize];
                let mut increment = 1.0 * (player.speed_factor as f32 / 1000.0);
                let mut inverted = false;
                for effect in &self.players[player_id as usize].effects {
                    if effect.malus.is_some() {
                        if effect.malus == Some(Malus::InvertedControls)
                        && !inverted {
                            inverted = true;
                            increment *= -1.0;
                        } else if effect.malus == Some(Malus::UltraFast) {
                            increment *= 4.0;
                        } else if effect.malus == Some(Malus::Slow) {
                            increment /= 4.0;
                        }
                    }
                }
                let mut x = player.x;
                let mut y = player.y;
                match direction {
                    Direction::North => y -= increment,
                    Direction::South => y += increment,
                    Direction::West => x -= increment,
                    Direction::East => x += increment,
                }
                if (x as i32) < 0 || (x as usize) >= self.map.w
                    || (y as i32) < 0 || (y as usize) >= self.map.h {
                    return;
                }
                let mut walkable = self.map.items[x as usize + y as usize * self.map.w].is_none();
                if !walkable {
                    walkable = self.map.items[x as usize + y as usize * self.map.w].as_ref().unwrap()
                        .walkable(player, &(x as usize, y as usize));
                }
                walkable &= self.map.squares[x as usize + y as usize * self.map.w].sq_type.walkable(player, &(x as usize, y as usize));
                if walkable {
                    player.x = x;
                    player.y = y;
                    let diff = PlayerMove {
                        msg_type: String::from("player_move_diff"),
                        id: player_id,
                        x,
                        y,
                    };
                    self.inform_players(&diff.to_vec());
                }
            }
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

    fn eat_bonus_and_malus(&mut self) {
        let mut pkts: Vec<Vec<u8>> = Vec::new();
        let mut idx = 0;
        for p in &mut self.map.players {
            if self.map.items[p.x as usize + self.map.w * p.y as usize].is_none() {
                idx += 1;
                continue;
            }
            let bonus = self.map.items[p.x as usize + self.map.w * p.y as usize].as_ref().unwrap().as_any().downcast_ref::<Bonus>();
            let mut inform = false;
            if bonus.is_some() {
                inform = true;
                match bonus.unwrap() {
                    Bonus::ImproveBombRadius => {
                        info!("Improve player {} bomb radius", idx);
                        p.radius += 1;
                    },
                    Bonus::PunchBombs => {
                        info!("Player {} can push bombs", idx);
                        self.players[idx as usize].effects.push(PlayerEffect {
                            end: None,
                            malus: None,
                            bonus: Some(Bonus::PunchBombs)
                        });
                        error!("TODO");
                    },
                    Bonus::ImproveSpeed => {
                        info!("Improve player {} speed", idx);
                        p.speed_factor += 100;
                    },
                    Bonus::RepelBombs => {
                        info!("Player {} can repel bombs", idx);
                        self.players[idx as usize].effects.push(PlayerEffect {
                            end: None,
                            malus: None,
                            bonus: Some(Bonus::RepelBombs)
                        });
                        error!("TODO");
                    },
                    Bonus::MoreBombs => {
                        info!("Player {} have more bombs", idx);
                        p.bomb += 1;
                    },
                    _ => {
                        error!("Unknown bonus");
                    },
                }
            }
            let malus = self.map.items[p.x as usize + self.map.w * p.y as usize].as_ref().unwrap().as_any().downcast_ref::<Malus>();
            if malus.is_some() {
                inform = true;
                match malus.unwrap() {
                    Malus::Slow => {
                        info!("Player {} is now slow", idx);
                        self.players[idx as usize].effects.push(PlayerEffect {
                            end: Some(Instant::now() + Duration::from_secs(10)),
                            malus: Some(Malus::Slow),
                            bonus: None,
                        });
                    },
                    Malus::UltraFast => {
                        info!("Player {} gotta go fast", idx);
                        self.players[idx as usize].effects.push(PlayerEffect {
                            end: Some(Instant::now() + Duration::from_secs(10)),
                            malus: Some(Malus::UltraFast),
                            bonus: None,
                        });
                    },
                    Malus::SpeedBomb => {
                        info!("Change player {} bomb' speed", idx);
                        self.players[idx as usize].effects.push(PlayerEffect {
                            end: Some(Instant::now() + Duration::from_secs(10)),
                            malus: Some(Malus::SpeedBomb),
                            bonus: None,
                        });
                        error!("TODO");
                    },
                    Malus::DropBombs => {
                        info!("Player {} drop bombs as fast as they can", idx);
                        self.players[idx as usize].effects.push(PlayerEffect {
                            end: Some(Instant::now() + Duration::from_secs(10)),
                            malus: Some(Malus::DropBombs),
                            bonus: None,
                        });
                        error!("TODO");
                    },
                    Malus::InvertedControls => {
                        info!("Player {} have inverted controls", idx);
                        self.players[idx as usize].effects.push(PlayerEffect {
                            end: Some(Instant::now() + Duration::from_secs(10)),
                            malus: Some(Malus::InvertedControls),
                            bonus: None,
                        });
                    },
                    _ => {
                        error!("Unknown malus");
                    },
                }
            }

            if inform {
                self.map.items[p.x as usize + self.map.w * p.y as usize] = None;
                let diff = DestroyItem {
                    msg_type: String::from("destroy_item"),
                    w: p.x as u64,
                    h: p.y as u64,
                };
                pkts.push(diff.to_vec());
            }
            idx += 1;
        }

        for pkt in pkts {
            self.inform_players(&pkt);
        }

        // Remove effects
        for p in &mut self.players {
            let mut idx = 0;
            for effect in p.effects.clone() {
                if effect.end.is_some() && effect.end.unwrap() < Instant::now() {
                    p.effects.remove(idx as usize);
                } else {
                    idx += 1;
                }
            }
        }
    }

    pub fn event_loop(&mut self) {
        self.execute_actions();
        self.eat_bonus_and_malus();
        // Explode bomb
        // TODO clean
        let mut bomb_idx = 0;
        let mut pkts = Vec::new();
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
                    let diff = BombExplode {
                        msg_type: String::from("bomb_explode"),
                        w: bomb.pos.0 as u64,
                        h: bomb.pos.1 as u64,
                    };
                    pkts.push(diff.to_vec());
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
                                                    self.map.items[pos.0 as usize + self.map.w * pos.1 as usize] = Some(bonus.box_clone());
                                                    let diff = CreateItem {
                                                        msg_type: String::from("create_item"),
                                                        item: Some(Box::new(bonus)),
                                                        w: pos.0 as u64,
                                                        h: pos.1 as u64,
                                                    };
                                                    pkts.push(diff.to_vec());
                                                } else if prob == 3 {
                                                    let malus : Malus = rand::random();
                                                    self.map.items[pos.0 as usize + self.map.w * pos.1 as usize] = Some(malus.box_clone());
                                                    let diff = CreateItem {
                                                        msg_type: String::from("create_item"),
                                                        item: Some(Box::new(malus)),
                                                        w: pos.0 as u64,
                                                        h: pos.1 as u64,
                                                    };
                                                    pkts.push(diff.to_vec());
                                                } else {
                                                    self.map.items[pos.0 as usize + self.map.w * pos.1 as usize] = None;

                                                    let diff = DestroyItem {
                                                        msg_type: String::from("destroy_item"),
                                                        w: pos.0 as u64,
                                                        h: pos.1 as u64,
                                                    };
                                                    pkts.push(diff.to_vec());
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
                                        if player.x as usize == pos.0 as usize && player.y as usize == pos.1 as usize {
                                            let diff = PlayerDie {
                                                msg_type: String::from("player_die"),
                                                id: p as u64,
                                            };
                                            pkts.push(diff.to_vec());
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

        for pkt in pkts {
            self.inform_players(&pkt);
        }
    }
}