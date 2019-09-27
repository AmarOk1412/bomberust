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

use std::fmt;
use rand::Rng;

use super::utils::{Direction, MapPlayer, Square, SquareType};
use super::item::*;

/**
 * Represent a map for a game
 */
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Map {
    pub w: usize,
    pub h: usize,
    pub squares: Vec<Square>,
    pub players: Vec<MapPlayer>,
    pub items: Vec<Option<InteractiveItem>>,
}

impl Map {
    /**
     * Generate a new map.
     * @todo redo and clean. Mostly avoid to generate players here
     * @param w     width of the map
     * @param h     height of the map
     * @return      The generated map
     */
    pub fn new(mut w: usize, mut h: usize) -> Map {
        if w < 11 {
            w = 11;
        }
        if h < 11 {
            h = 11;
        }
        let size = (w * h) as usize;
        let mut squares = Vec::with_capacity(size);
        let mut items: Vec<Option<InteractiveItem>> = Vec::with_capacity(size);
        let mut players = Vec::new();
        let mut x = 0;
        let mut y = 0;
        let mut rng = rand::thread_rng();
        for _ in 0..size {
            let mut sq_type = rand::random();
            if x % 2 == 1 && y % 2 == 1 {
                sq_type = SquareType::Block;
            }

            let add_box: u8 = rng.gen();
            if add_box % 3 != 0 && sq_type == SquareType::Empty {
                items.push(Some(Box::new(DestructibleBox {})));
            } else {
                items.push(None);
            }
            squares.push(Square {
                sq_type
            });

            // Next square
            x += 1;
            x %= w;
            if x == 0 {
                y += 1;
            }
        }
        // Generate players
        for p in 0..4 {
            let mut valid_pos = false;
            let mut posx: usize = 0;
            let mut posy: usize = 0;
            let mut player = MapPlayer {
                x: 0.5,
                y: 0.5,
                radius: 2,
                speed_factor: 1.0,
                bomb: 1,
            };
            while !valid_pos {
                let random_x : usize = rng.gen();
                let random_y : usize = rng.gen();
                posx = random_x % (w / 4);
                posy = random_y % (h / 4);
                if p == 1 || p == 3 {
                    posx = w - posx - 1;
                }
                if p == 2 || p == 3 {
                    posy = h - posy - 1;
                }
                player.x = posx as f32 + 0.5;
                player.y = posy as f32 + 0.5;
                if squares[posx + posy * w].sq_type.walkable(&player, &(posx, posy)) {
                    valid_pos = true;
                }
            }
            items[posx + posy * w] = None;
            players.push(player);
        }
        let mut res = Map {
            w,
            h,
            squares,
            players,
            items
        };
        res.make_startable();
        res
    }

    /**
     * Modify the map till all players can safely play
     * @todo REDO THIS DIRTY AND HACKY THING
     */
    fn make_startable(&mut self) {
        for p in &self.players {
            let mut rng = rand::thread_rng();
            let mut different_x = false;
            let mut different_y = false;
            let mut destroyable: Vec<(usize, usize)> = Vec::new();
            let mut safe: Vec<(usize, usize)> = Vec::new();
            safe.push((p.x as usize, p.y as usize));

            let mut safe_idx = 0;
            let mut prefer_n: bool = rng.gen();
            let mut prefer_w: bool = rng.gen();
            let mut check_x = true;
            let mut inc_x: i32 = 0;
            let mut inc_y: i32 = 0;
            let mut direction_tested: u8 = 0;
            while !different_x || !different_y {
                if direction_tested == 4 {
                    direction_tested = 0;
                    prefer_n = rng.gen();
                    prefer_w = rng.gen();
                    let current = safe[safe_idx].clone();
                    safe_idx += 1;
                    if safe_idx >= safe.len() {
                        if destroyable.len() == 0 {
                            break;
                        }
                        let new_safe = destroyable.pop().unwrap();
                        let linearized_pos = new_safe.0 + new_safe.1 * self.w;
                        let walkable_item = match &self.items[linearized_pos] {
                            Some(i) => i.walkable(p, &(new_safe)),
                            None => true
                        };
                        safe.push(new_safe);
                        if !walkable_item {
                            self.items[linearized_pos] = None;
                        } else {
                            self.squares[linearized_pos].sq_type = SquareType::Empty;
                        }
                        if new_safe.0 != current.0 {
                            different_x = true;
                        } else if new_safe.1 != current.1 {
                            different_y = true;
                        }
                    }
                }
                if check_x {
                    if prefer_w {
                        inc_x -= 1;
                    } else {
                        inc_x += 1;
                    }
                } else {
                    if prefer_n {
                        inc_y -= 1;
                    } else {
                        inc_y += 1;
                    }
                }
                let to_test_x: i32 = safe[safe_idx].0 as i32 + inc_x;
                let to_test_y: i32 = safe[safe_idx].1 as i32 + inc_y;
                if to_test_x < 0 || to_test_x >= self.w as i32 {
                    inc_x = 0;
                    check_x = !check_x;
                    direction_tested += 1;
                    prefer_w = !prefer_w;
                    continue;
                }
                if to_test_y < 0 || to_test_y >= self.h as i32 {
                    inc_y = 0;
                    check_x = !check_x;
                    direction_tested += 1;
                    prefer_n = !prefer_n;
                    continue;
                }
                if safe.iter().find(|&&x| x == (to_test_x as usize, to_test_y as usize)) != None {
                    if check_x {
                        inc_x = 0;
                        prefer_w = !prefer_w;
                    } else {
                        inc_y = 0;
                        prefer_n = !prefer_n;
                    }
                    check_x = !check_x;
                    direction_tested += 1;
                    continue;
                }
                let linearized_pos = to_test_x as usize + to_test_y as usize * self.w;
                let walkable_item = match &self.items[linearized_pos] {
                    Some(i) => i.walkable(p, &(to_test_x as usize, to_test_y as usize)),
                    None => true
                };
                if self.squares[linearized_pos].sq_type.walkable(p, &(to_test_x as usize, to_test_y as usize)) && walkable_item {
                    safe.push((to_test_x as usize, to_test_y as usize));
                    if check_x {
                        different_x = true;
                    } else {
                        different_y = true;
                    }
                } else {
                    if !walkable_item || self.squares[linearized_pos].sq_type != SquareType::Block {
                        destroyable.push((to_test_x as usize, to_test_y as usize));
                    }
                }

                if check_x {
                    inc_x = 0;
                    check_x = !check_x;
                    direction_tested += 1;
                    prefer_w = !prefer_w;
                } else {
                    inc_y = 0;
                    check_x = !check_x;
                    direction_tested += 1;
                    prefer_n = !prefer_n;
                }
            }
        }
    }
}

impl fmt::Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut map_str = String::new();
        let mut x = 0;
        for sq in &self.squares {
            // Test if it's a player
            let mut is_player_here = false;
            for p in &self.players {
                if (p.x as usize + p.y as usize * self.w) == x {
                    is_player_here = true;
                }
            }
            // Draw square
            match sq.sq_type {
                SquareType::Water => map_str.push('W'),
                SquareType::Empty => {
                    match &self.items[x] {
                        Some(i) => {
                            if i.name() == "DestructibleBox" {
                                map_str.push('D');
                            } else if i.name() == "Bomb" {
                                if is_player_here {
                                    is_player_here = false;
                                    map_str.push('p');
                                } else {
                                    map_str.push('b');
                                }
                            } else if i.name() == "Bonus" {
                                map_str.push('O');
                            } else if i.name() == "Malus" {
                                map_str.push('M');
                            } else {
                                map_str.push('u');
                            }
                        }
                        _ => map_str.push('X')
                    }
                },
                SquareType::Block => map_str.push('B'),
                SquareType::Wall(d) => {
                    match d {
                        Direction::North => map_str.push('N'),
                        Direction::South => map_str.push('S'),
                        Direction::West  => map_str.push('W'),
                        Direction::East  => map_str.push('E'),
                    }
                }
            }
            if is_player_here {
                map_str.pop();
                map_str.push('P');
            }
            x += 1;
            if x % self.w == 0 {
                map_str.push('\n');
            }
        }
        write!(f, "{}", map_str)
    }
}