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

use super::super::item::Walkable;
use super::{Direction, MapPlayer};
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};


#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
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

impl Walkable for SquareType {
    fn walkable(&self, p: &MapPlayer, pos: &(usize, usize)) -> bool {
        match self {
            SquareType::Empty => true,
            SquareType::Wall(w) => {
                match w {
                    Direction::West => p.x as usize >= pos.0,
                    Direction::East => p.x as usize <= pos.0,
                    Direction::North => p.y as usize <= pos.1,
                    Direction::South => p.y as usize >= pos.1,
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

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct Square {
    pub sq_type: SquareType
}