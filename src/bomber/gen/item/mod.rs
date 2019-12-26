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

use super::utils::MapPlayer;

use std::any::Any;
use std::fmt::Debug;
use serde::Serialize;
use rmps::Serializer;

pub trait Walkable {
    fn walkable(&self, p: &MapPlayer, pos: &(f32, f32)) -> bool;

    fn explode_event(&self, pos: &(usize, usize), bomb_pos: &(usize, usize)) -> (bool /* block */, bool /* destroy item */);
}

#[typetag::serde]
pub trait Item: Walkable + Sync + Send + Debug {
    fn name(&self) -> String;

    fn as_any(&self) -> &dyn Any;

    fn box_clone(&self) -> Box<dyn Item>;
}

pub type InteractiveItem = Box<dyn Item>;

/**
 * Compare if serialized objects are identical
 */
impl PartialEq for Box<dyn Item> {
    fn eq(&self, other: &Self) -> bool {
        let mut buf = Vec::new();
        let mut buf_other = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        other.serialize(&mut Serializer::new(&mut buf_other)).unwrap();
        buf == buf_other
    }
}

impl Clone for Box<dyn Item>
{
    fn clone(&self) -> Box<dyn Item> {
        self.box_clone()
    }
}

pub mod bomb;
pub use bomb::BombItem;
pub mod bonus;
pub use bonus::Bonus;
pub mod destructiblebox;
pub use destructiblebox::DestructibleBox;
pub mod malus;
pub use malus::Malus;
