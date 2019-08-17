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
use std::time::Duration;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

pub enum Shape {
    Cross,
    Square,
    Circle
}

pub struct Bomb {
    pub radius: usize,
    pub shape: Shape,
    pub created_time: u64,
    pub duration: Duration
}

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

#[derive(Clone)]
pub struct Player {
}

pub trait Item {
    fn walkable(&self, p: Player) -> bool;
}

impl Item for Bomb {
    fn walkable(&self, _p: Player) -> bool {
        false
    }
}

pub struct DestructibleBox {
    
}

impl Item for DestructibleBox {
    fn walkable(&self, _p: Player) -> bool {
        false
    }
}

type InteractiveItem = Box<dyn Item>;

pub struct Map {
    pub w: usize,
    pub h: usize,
    pub squares: Vec<Square>,
    pub players: Vec<Player>,
    pub items: Vec<Option<InteractiveItem>>,

}

impl Map {
    pub fn new(w: usize, h: usize) -> Map {
        let size = (w * h) as usize;
        let mut squares = Vec::with_capacity(size);
        let mut items: Vec<Option<InteractiveItem>> = Vec::with_capacity(size);
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
        Map {
            w,
            h,
            squares,
            players: Vec::new(),
            items
        }
    }
}

impl fmt::Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut map_str = String::new();
        let mut x = 0;
        for sq in &self.squares {
            match sq.sq_type {
                SquareType::Water => map_str.push('W'),
                SquareType::Empty => {
                    match self.items[x] {
                        Some(_) => map_str.push('D'),
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
            x += 1;
            if x % self.w == 0 {
                map_str.push('\n');
            }
        }
        write!(f, "{}", map_str)
    }
}
