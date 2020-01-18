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
use std::collections::{ HashMap, HashSet, VecDeque };
use std::time::{Duration, Instant};
use std::f64::consts::PI;

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
    blocked_pos: HashSet<(i32, i32)>,
    exploding_pos: HashSet<(i32, i32)>
}

#[derive(Clone)]
pub struct Bomb {
    pub creator_id: u32,
    pub radius: usize,
    pub shape: Shape,
    pub created_time: Instant,
    pub duration: Duration,
    pub pos: (f32, f32),
    pub exploding_info: Option<ExplodingInfo>,
    pub moving_dir: Option<Direction>
}

pub struct Game {
    pub map: Map,
    pub players: Vec<GamePlayer>,
    pub bombs: Vec<Bomb>,
    pub game_player_to_player: HashMap<u64, Player>,
    started: Instant,
    duration: Duration,
    players_len: u32,
    last_printed: Instant,
    last_update_bomb: Instant,
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
            players_len: 4,
            bombs: Vec::new(),
            game_player_to_player: HashMap::new(),
            started: Instant::now(),
            duration: Duration::from_secs(60 * 3),
            last_printed: Instant::now(),
            last_update_bomb: Instant::now(),
            fps_instants: VecDeque::new(),
        }
    }

    pub fn start(&mut self) {
        self.started = Instant::now();
        self.last_printed = Instant::now();
        self.last_update_bomb = Instant::now();
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
        let diff = PlayerIdentity {
            msg_type: String::from("player_identity"),
            id,
        };
        player.rx.lock().unwrap().push(diff.to_vec());
        self.game_player_to_player.insert(id, player);

        Some(id)
    }

    fn inform_players(&mut self, diff: &Vec<u8>) {
        for (_, player) in &mut self.game_player_to_player {
            player.rx.lock().unwrap().push(diff.clone());
        }
    }

    pub fn finished(&self) -> bool {
        let mut deads = 0;
        let mut idx = 0;
        for p in &self.map.players {
            if p.dead
            && idx < self.game_player_to_player.len() /* linked */ {
                deads += 1;
            }
            idx += 1;
        }
        deads >= self.game_player_to_player.len()
    }

    fn execute(&mut self, action: Action, player_id: i32) {
        if self.map.players[player_id as usize].dead {
            return;
        }
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
                let mut bomb_duration = Duration::from_secs(3);
                for effect in &self.players[player_id as usize].effects {
                    if effect.malus.is_some() {
                        if effect.malus == Some(Malus::SpeedBomb) {
                            bomb_duration = Duration::from_secs_f32(1.8);
                        }
                    }
                }
                self.map.items[player.x as usize + player.y as usize * self.map.w] = Some(Box::new(BombItem {}));
                self.bombs.push(Bomb {
                    creator_id: player_id as u32,
                    radius: player.radius as usize,
                    shape: Shape::Cross,
                    created_time: Instant::now(),
                    duration: bomb_duration,
                    pos: (player.x as usize as f32 + 0.5, player.y as usize as f32 + 0.5),
                    exploding_info: None,
                    moving_dir: None,
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
                let player = &mut self.map.players[player_id as usize];
                let mut increment = 0.1 * (player.speed_factor as f32 / 1000.0);
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
                    // Manage bomb repelling if player is near a bomb
                    if self.map.items[x as usize + y as usize * self.map.w].as_ref().unwrap().name() == "Bomb" {
                        for effect in &self.players[player_id as usize].effects {
                            if effect.bonus == Some(Bonus::RepelBombs) {
                                for b in &mut self.bombs {
                                    if b.moving_dir.is_none() && b.pos.0 as usize == x as usize && b.pos.1 as usize == y as usize {
                                        if player_id as usize == b.creator_id as usize && b.pos.0 as usize == player.x as usize && b.pos.1 as usize == player.y as usize {
                                            // New bomb
                                            continue;
                                        }
                                        b.moving_dir = Some(direction);
                                    }
                                }
                            }
                        }
                    }
                    walkable = self.map.items[x as usize + y as usize * self.map.w].as_ref().unwrap()
                        .walkable(player, &(x, y));
                }
                walkable &= self.map.squares[x as usize + y as usize * self.map.w].sq_type.walkable(player, &(x, y));
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
            // Add actions generated by bonus/malus
            for effect in &p.effects {
                if effect.malus.is_some() && p.actions.len() == 0 {
                    if effect.malus == Some(Malus::DropBombs) {
                        p.actions.push(Action::PutBomb);
                    }
                }
            }

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
            if p.dead {
                idx += 1;
                continue;
            }
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
            let malus = self.map.items[p.x as usize + self.map.w * p.y as usize]
                .as_ref().unwrap().as_any().downcast_ref::<Malus>();
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
                    },
                    Malus::DropBombs => {
                        info!("Player {} drop bombs as fast as they can", idx);
                        self.players[idx as usize].effects.push(PlayerEffect {
                            end: Some(Instant::now() + Duration::from_secs(10)),
                            malus: Some(Malus::DropBombs),
                            bonus: None,
                        });
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

    fn update_exploding_radius(&mut self, bomb_idx: usize) -> Option<i32> {
        let mut bomb = &mut self.bombs[bomb_idx];

        // Explode
        let mut exploding_radius = 0;
        if bomb.exploding_info.is_none() {
            bomb.exploding_info = Some(ExplodingInfo {
                radius: 0,
                exploding_time: Instant::now(),
                blocked_pos: HashSet::new(),
                exploding_pos: HashSet::new(),
            });
            let diff = BombExplode {
                msg_type: String::from("bomb_explode"),
                w: bomb.pos.0 as u64,
                h: bomb.pos.1 as u64,
            };
            self.inform_players(&diff.to_vec());
        } else {
            exploding_radius = bomb.exploding_info.as_ref().unwrap().radius;
            if exploding_radius == bomb.radius as i32 {
                // End of the bomb
                self.map.items[bomb.pos.0 as usize + self.map.w * bomb.pos.1 as usize] = None;
                self.bombs.remove(bomb_idx);
                return None;
            }

            let exploding_time = bomb.exploding_info.as_ref().unwrap().exploding_time;
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
        Some(exploding_radius)
    }

    fn update_exploding_pos(&mut self, bomb_idx: usize) {
        let bomb = &mut self.bombs[bomb_idx];
        let radius = bomb.exploding_info.as_ref().unwrap().radius;
        for r in 0..(radius + 1) {
            // TODO other than Cross
            for angle in (0..360).step_by(90) {
                let angle = (angle as f64 / 360.) * (2. * PI);
                let x = bomb.pos.0 as i32 + (angle.cos() as i32 * r);
                let y = bomb.pos.1 as i32 + (angle.sin() as i32 * r);
                if x < 0 || x >= self.map.w as i32 || y < 0 || y >= self.map.h as i32 {
                    continue;
                }
                let pos = (x, y);
                if bomb.exploding_info.as_ref().unwrap().blocked_pos.contains(&pos) {
                    continue; // Already checked
                }
                let pos = (x as usize, y as usize);
                // Check if path to the bomb is blocked
                let mut blocked = false;
                for pr in 0..r {
                    let px = bomb.pos.0 as i32 + (angle.cos() as i32 * pr);
                    let py = bomb.pos.1 as i32 + (angle.sin() as i32 * pr);
                    if bomb.exploding_info.as_ref().unwrap().blocked_pos.contains(&(px, py)) {
                        blocked = true;
                        break;
                    }
                }
                if blocked {
                    continue;
                }
                let (block, clear) = self.map.squares[pos.0 + self.map.w * pos.1]
                    .sq_type.explode_event(&(pos.0, pos.1), &(bomb.pos.0 as usize, bomb.pos.1 as usize));
                blocked = block;
                if !blocked {
                    let item = &self.map.items[pos.0 + self.map.w * pos.1];
                    if item.is_some() {
                        // TODO as_ref for moving blocked_pos
                        let (iblock, _) = item.as_ref().unwrap()
                            .explode_event(&pos, &(bomb.pos.0 as usize, bomb.pos.1 as usize));
                        blocked = iblock;
                    }
                }
                // Check if clear square
                if clear {
                    bomb.exploding_info.as_mut().unwrap().exploding_pos.insert((pos.0 as i32, pos.1 as i32));
                }
                // Check if bomb is stopped
                if blocked && clear {
                    let nxt_x = bomb.pos.0 as i32 + (angle.cos() as i32 * (r + 1));
                    let nxt_y = bomb.pos.1 as i32 + (angle.sin() as i32 * (r + 1));
                    bomb.exploding_info.as_mut().unwrap().blocked_pos.insert((nxt_x, nxt_y));
                } else if blocked {
                    bomb.exploding_info.as_mut().unwrap().blocked_pos.insert((pos.0 as i32, pos.1 as i32));
                }
            }
        }
    }

    fn kill_players(&mut self, x: i32, y: i32) {
        let mut pkts = Vec::new();
        let mut p = 0;
        for player in &mut self.map.players {
            if !player.dead
            && player.x as usize == x as usize
            && player.y as usize == y as usize {
                let diff = PlayerDie {
                    msg_type: String::from("player_die"),
                    id: p as u64,
                };
                pkts.push(diff.to_vec());
                player.dead = true;
            }
            p += 1;
        }
        for pkt in pkts {
            self.inform_players(&pkt);
        }
    }

    fn remove_item(&mut self, x: i32, y: i32) {
        self.map.items[x as usize + self.map.w * y as usize] = None;

        let diff = DestroyItem {
            msg_type: String::from("destroy_item"),
            w: x as u64,
            h: y as u64,
        };
        self.inform_players(&diff.to_vec());
    }

    fn bomb_events(&mut self) {
        let mut explodings_bombs_idx = Vec::new();
        for bomb_idx in 0..self.bombs.len() {
            let bomb = &mut self.bombs[bomb_idx];
            // Check if exploding
            if Instant::now() - bomb.duration < bomb.created_time {
                continue;
            }
            explodings_bombs_idx.push(bomb_idx);
        }
        let mut pkts = Vec::new();
        for bomb_idx in explodings_bombs_idx {
            // Explode bomb
            let exploding_radius = self.update_exploding_radius(bomb_idx);
            if exploding_radius.is_none() {
                continue;
            }
            self.update_exploding_pos(bomb_idx);

            for (x, y) in self.bombs[bomb_idx].exploding_info.as_ref().unwrap().exploding_pos.clone() {
                self.kill_players(x, y);
                // Destroy items in zone
                let item = &self.map.items[x as usize + self.map.w * y as usize];
                if item.is_some() {
                    let db = self.map.items[x as usize + self.map.w * y as usize].as_ref().unwrap().as_any().downcast_ref::<DestructibleBox>();
                    match db {
                        Some(_) => {
                            let mut rng = rand::thread_rng();
                            let prob = rng.gen_range(0, 5);
                            if prob == 1 || prob == 2 {
                                let bonus : Bonus = rand::random();
                                self.map.items[x as usize + self.map.w * y as usize] = Some(bonus.box_clone());
                                let diff = CreateItem {
                                    msg_type: String::from("create_item"),
                                    item: Some(Box::new(bonus)),
                                    w: x as u64,
                                    h: y as u64,
                                };
                                pkts.push(diff.to_vec());
                            } else if prob == 3 {
                                let malus : Malus = rand::random();
                                self.map.items[x as usize + self.map.w * y as usize] = Some(malus.box_clone());
                                let diff = CreateItem {
                                    msg_type: String::from("create_item"),
                                    item: Some(Box::new(malus)),
                                    w: x as u64,
                                    h: y as u64,
                                };
                                pkts.push(diff.to_vec());
                            } else {
                                self.remove_item(x, y);
                            }
                        },
                        _ => {}
                    }
                }
            }
        };

        for pkt in pkts {
            self.inform_players(&pkt);
        }
    }

    pub fn update_end_anim(&mut self) {
        if self.started + self.duration - Duration::from_secs(30) <= Instant::now() {
            let duration_left = self.started + self.duration - Instant::now();
            let squares_nb = self.map.h * self.map.w;
            let interval = Duration::from_secs(30).as_millis() / squares_nb as u128;
            let square = (
                (
                    (Duration::from_secs(30).as_millis() - duration_left.as_millis()) as f32
                    / Duration::from_secs(30).as_millis() as f32
                ) * interval as f32
            ) as u32;

            // TODO more animations

            let x = square as usize % self.map.w;
            let y = square as usize / self.map.w;
            let linearized =
                if y % 2 == 0 { (y / 2) * self.map.w + x }
                else { squares_nb - 1 - ( y / 2 ) * self.map.w - x  };

            if self.map.squares[linearized].sq_type == SquareType::Block {
                return;
            }

            let x = (linearized % self.map.w) as u64;
            let y = (linearized / self.map.w) as u64;

            // The square is now a block
            self.map.squares[linearized].sq_type = SquareType::Block;
            let diff = UpdateSquare {
                msg_type: String::from("update_square"),
                square: SquareType::Block,
                x,
                y,
            };
            self.inform_players(&diff.to_vec());
            self.remove_item(x as i32, y as i32);
            self.kill_players(x as i32, y as i32);

        }
    }

    fn update_bomb_position(&mut self) {
        // The speed should be 2 squares/seconds
        if self.last_update_bomb + Duration::from_millis(50) > Instant::now() {
            return;
        }
        let mut diffs = Vec::<BombMove>::new();
        for b in &mut self.bombs {
            if b.moving_dir.is_none() || b.exploding_info.is_some() {
                continue;
            }
            let direction = b.moving_dir.unwrap();
            let increment = 0.1;
            let mut x = b.pos.0;
            let mut y = b.pos.1;
            match direction {
                Direction::North => y -= increment,
                Direction::South => y += increment,
                Direction::West => x -= increment,
                Direction::East => x += increment,
            }
            if (x as i32) < 0 || (x as usize) >= self.map.w
                || (y as i32) < 0 || (y as usize) >= self.map.h {
                continue;
            }
            let mut walkable = x as usize == b.pos.0 as usize && y as usize == b.pos.1 as usize;
            if !walkable {
                let fake_player = MapPlayer {
                    x: b.pos.0, y: b.pos.1, radius: 0, speed_factor: 0, bomb: 0, dead: false,
                };
                // Items should not be a bomb or a box
                walkable = self.map.items[x as usize + y as usize * self.map.w].is_none()
                         || self.map.items[x as usize + y as usize * self.map.w].as_ref().unwrap().walkable(&fake_player, &(x, y));
                // Else square should be empty
                walkable &= self.map.squares[x as usize + y as usize * self.map.w].sq_type.walkable(&fake_player, &(x, y));
                // Players should not be here
                for p in &self.map.players {
                    if p.dead {
                        continue;
                    }
                    if p.x as usize == x as usize && p.y as usize == y as usize {
                        walkable &= false;
                    }
                }
            }
            if (x as i32) < 0 || (x as usize) >= self.map.w
                || (y as i32) < 0 || (y as usize) >= self.map.h {
                b.moving_dir = None;
                continue;
            }
            if walkable {
                if b.pos.0 as usize != x as usize || b.pos.1 as usize != y as usize {
                    self.map.items[x as usize + y as usize * self.map.w] = Some(Box::new(BombItem {}));
                    self.map.items[b.pos.0 as usize + b.pos.1 as usize * self.map.w] = None;
                }
                diffs.push(BombMove {
                    msg_type: String::from("bomb_move_diff"),
                    old_x: b.pos.0,
                    old_y: b.pos.1,
                    x,
                    y,
                });
                b.pos = (x,y);
            } else {
                b.moving_dir = None;
            }
        }
        for diff in diffs {
            self.inform_players(&diff.to_vec());
        }
        self.last_update_bomb = Instant::now();
    }

    pub fn event_loop(&mut self) {
        self.execute_actions();
        self.eat_bonus_and_malus();
        self.bomb_events();
        self.update_end_anim();
        self.update_bomb_position();
    }
}