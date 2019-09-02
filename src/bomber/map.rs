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

use std::any::Any;
use std::fmt;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

pub struct BombItem {}

#[derive(Clone, Copy, PartialEq)]
pub enum Direction {
    North,
    South,
    West,
    East,
}

impl Distribution<Direction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Direction {
        match rng.gen_range(0, 4) {
            0 => Direction::North,
            1 => Direction::South,
            2 => Direction::West,
            _ => Direction::East,
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum SquareType {
    Water,
    Empty,
    Wall(Direction),
    Block, /* Not randomly generated */
}

impl Distribution<SquareType> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SquareType {
        match rng.gen_range(0, 22) {
            0 => SquareType::Water,
            1 => SquareType::Wall(rand::random()),
            _ => SquareType::Empty,
        }
    }
}

#[derive(Clone)]
pub struct Square {
    pub sq_type: SquareType
}

#[derive(Clone, Copy)]
pub struct MapPlayer {
    pub x: usize,
    pub y: usize
}

pub trait Walkable {
    fn walkable(&self, p: &MapPlayer, pos: &(usize, usize)) -> bool;

    fn explode_event(&self, pos: &(usize, usize), bomb_pos: &(usize, usize)) -> (bool /* block */, bool /* destroy item */);
}


pub trait Item: Walkable + Sync + Send {
    fn name(&self) -> String;

    fn as_any(&self) -> &dyn Any;
}

impl Walkable for BombItem {
    fn walkable(&self, _p: &MapPlayer, _pos: &(usize, usize)) -> bool {
        false
    }

    fn explode_event(&self, pos: &(usize, usize), bomb_pos: &(usize, usize)) -> (bool, bool) {
        (bomb_pos.0 != pos.0 || bomb_pos.1 != pos.1, true)
    }
}

pub struct DestructibleBox {
    
}

impl Item for BombItem {
    // TODO better solution?
    fn name(&self) -> String {
        String::from("Bomb")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Walkable for DestructibleBox {
    fn walkable(&self, _p: &MapPlayer, _pos: &(usize, usize)) -> bool {
        false
    }

    fn explode_event(&self, _pos: &(usize, usize), _bomb_pos: &(usize, usize)) -> (bool, bool) {
        (true, true)
    }
}

impl Item for DestructibleBox {
    fn name(&self) -> String {
        String::from("DestructibleBox")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

type InteractiveItem = Box<dyn Item>;

impl Walkable for SquareType {
    fn walkable(&self, p: &MapPlayer, pos: &(usize, usize)) -> bool {
        match self {
            SquareType::Empty => true,
            SquareType::Wall(w) => {
                match w {
                    Direction::West => p.x >= pos.0,
                    Direction::East => p.x <= pos.0,
                    Direction::North => p.y <= pos.1,
                    Direction::South => p.y >= pos.1,
                }
            },
            _ => false
        }
    }

    fn explode_event(&self, pos: &(usize, usize), bomb_pos: &(usize, usize)) -> (bool, bool) {
        match self {
            SquareType::Empty => (false, true),
            SquareType::Water => (false, false),
            SquareType::Block => (true, false),
            SquareType::Wall(w) => {
                match w {
                    Direction::North => (bomb_pos.1 == pos.1, bomb_pos.1 <= pos.1),
                    Direction::South => (bomb_pos.1 == pos.1, bomb_pos.1 >= pos.1),
                    Direction::West => (bomb_pos.0 == pos.0, bomb_pos.0 >= pos.0),
                    Direction::East => (bomb_pos.0 == pos.0, bomb_pos.0 <= pos.0),
                }
            },
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum Bonus {
    ImproveBombRadius,
    PunchBombs,
    ImproveSpeed,
    RepelBombs,
    MoreBombs,
    Custom(String)
}

impl Distribution<Bonus> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Bonus {
        match rng.gen_range(0, 4) {
            0 => Bonus::ImproveBombRadius,
            1 => Bonus::PunchBombs,
            2 => Bonus::ImproveSpeed,
            3 => Bonus::RepelBombs,
            _ => Bonus::MoreBombs,
        }
    }
}

impl Walkable for Bonus {
    fn walkable(&self, _p: &MapPlayer, _pos: &(usize, usize)) -> bool {
        true
    }

    fn explode_event(&self, _pos: &(usize, usize), _bomb_pos: &(usize, usize)) -> (bool, bool) {
        (true, true)
    }
}

impl Item for Bonus {
    fn name(&self) -> String {
        String::from("Bonus")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone, PartialEq)]
pub enum Malus {
    Slow,
    UltraFast,
    SpeedBomb,
    DropBombs,
    InvertedControls,
    Custom(String)
}

impl Distribution<Malus> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Malus {
        match rng.gen_range(0, 4) {
            0 => Malus::Slow,
            1 => Malus::UltraFast,
            2 => Malus::SpeedBomb,
            3 => Malus::DropBombs,
            _ => Malus::InvertedControls,
        }
    }
}

impl Walkable for Malus {
    fn walkable(&self, _p: &MapPlayer, _pos: &(usize, usize)) -> bool {
        true
    }

    fn explode_event(&self, _pos: &(usize, usize), _bomb_pos: &(usize, usize)) -> (bool, bool) {
        (true, true)
    }
}

impl Item for Malus {
    fn name(&self) -> String {
        String::from("Malus")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}


pub struct Map {
    pub w: usize,
    pub h: usize,
    pub squares: Vec<Square>,
    pub players: Vec<MapPlayer>,
    pub items: Vec<Option<InteractiveItem>>,

}

impl Map {
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
                x: 0,
                y: 0
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
                player.x = posx;
                player.y = posy;
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

    fn make_startable(&mut self) {
        for p in &self.players {
            let mut rng = rand::thread_rng();
            let mut different_x = false;
            let mut different_y = false;
            let mut destroyable: Vec<(usize, usize)> = Vec::new();
            let mut safe: Vec<(usize, usize)> = Vec::new();
            safe.push((p.x, p.y));

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
                if (p.x + p.y * self.w) == x {
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