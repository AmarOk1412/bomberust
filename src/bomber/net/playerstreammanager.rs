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

use super::msg::*;
use super::super::core::Server;
use super::super::gen::utils::Direction;

use rmps::{ Serializer, Deserializer };
use rmps::decode::Error;
use serde::{ Serialize, Deserialize };
use std::io::Cursor;
use std::sync::{ Arc, Mutex };
use std::time::{ Duration, Instant };

/**
 * Contains the current datas
 */
pub struct RtpBuf {
    data: [u8; 65536],
    size: u16,
    wanted: u16,
}

/**
 * Wrap the TLS socket and current incoming datas
 */
pub struct Stream {
    id: u64,
    pub data: Arc<Mutex<Option<Vec<u8>>>>,
    rtp_buf: RtpBuf,
    last_pkt: Instant,
}

/**
 * Manager incoming streams and pass events to the Server
 */
pub struct PlayerStreamManager {
    current_id: u64,
    pub streams: Vec<Stream>,
    pub server: Arc<Mutex<Server>>,
}

impl PlayerStreamManager {
    /**
     * Generate a new PlayerStreamManager
     */
    pub fn new(server: Arc<Mutex<Server>>) -> PlayerStreamManager {
        PlayerStreamManager {
            current_id: 0,
            streams: Vec::new(),
            server,
        }
    }

    /**
     * Add a stream to process
     * @param stream    The stream to add
     * @return          The stream id
     */
    pub fn add_stream(&mut self) -> u64 {
        let id = self.current_id;
        let data = Arc::new(Mutex::new(None));
        self.streams.push(Stream {
            id,
            data: data.clone(),
            rtp_buf: RtpBuf {
                data: [0; 65536],
                size: 0,
                wanted: 0,
            },
            last_pkt: Instant::now()
        });
        self.current_id += 1;
        self.server.lock().unwrap().join_server(id, data);
        id
    }

    /**
     * Each packets are wrapped in a msgpack object.
     * This function deserialize the message and execute the action.
     * @note: TODO verify signature
     * @param pkt   The packet to process
     * @param id    The stream id
     */
    fn parse_pkt(&mut self, pkt: Vec<u8>, id: u64) {
        debug!("rx:{}", pkt.len());
        let cur = Cursor::new(&*pkt);
        let mut de = Deserializer::new(cur);
        let actual: Result<Msg, Error> = Deserialize::deserialize(&mut de);
        if actual.is_ok() {
            let msg_type = actual.unwrap().msg_type;
            let cur = Cursor::new(&*pkt);
            let mut de = Deserializer::new(cur);
            if msg_type == "create" {
                let new_room_id = self.server.lock().unwrap().create_room(id);
                if new_room_id != 0 {
                    // Announce to player that they join the room
                    let mut buf = Vec::new();
                    let msg = JoinedMsg::new(new_room_id, true);
                    msg.serialize(&mut Serializer::new(&mut buf)).unwrap();
                    let len = buf.len() as u16;
                    let mut send_buf : Vec<u8> = Vec::with_capacity(65536);
                    send_buf.push((len >> 8) as u8);
                    send_buf.push((len as u16 % (2 as u16).pow(8)) as u8);
                    send_buf.append(&mut buf);

                    for stream in &self.streams {
                        if stream.id == id {
                            *stream.data.lock().unwrap() = Some(send_buf.clone());
                            break;
                        }
                    }
                }
            } else if msg_type == "leave" {
                let success = self.server.lock().unwrap().leave_room(id);
                // Announce to player that they join the room
                let mut buf = Vec::new();
                let msg = JoinedMsg::new(0, success);
                msg.serialize(&mut Serializer::new(&mut buf)).unwrap();
                let len = buf.len() as u16;
                let mut send_buf : Vec<u8> = Vec::with_capacity(65536);
                send_buf.push((len >> 8) as u8);
                send_buf.push((len as u16 % (2 as u16).pow(8)) as u8);
                send_buf.append(&mut buf);

                for stream in &self.streams {
                    if stream.id == id {
                        *stream.data.lock().unwrap() = Some(send_buf.clone());
                        break;
                    }
                }
            } else if msg_type == "join" {
                let msg: JoinMsg = Deserialize::deserialize(&mut de).unwrap_or(JoinMsg::new(0));
                let success = self.server.lock().unwrap().join_room(id, msg.room);
                // Announce to player that they join the room
                let mut buf = Vec::new();
                let msg = JoinedMsg::new(id, success);
                msg.serialize(&mut Serializer::new(&mut buf)).unwrap();
                let len = buf.len() as u16;
                let mut send_buf : Vec<u8> = Vec::with_capacity(65536);
                send_buf.push((len >> 8) as u8);
                send_buf.push((len as u16 % (2 as u16).pow(8)) as u8);
                send_buf.append(&mut buf);

                for stream in &self.streams {
                    if stream.id == id {
                        *stream.data.lock().unwrap() = Some(send_buf.clone());
                        break;
                    }
                }
            } else if msg_type == "launch" {
                self.server.lock().unwrap().launch_game(id);
            } else {
                // In game action
                for s in &mut self.streams {
                    // Anti flood a packet have 10 ms delay.
                    if s.id == id {
                        if s.last_pkt + Duration::from_millis(10) > Instant::now() {
                            return;
                        } else {
                            s.last_pkt = Instant::now()
                        }
                    }
                }
                if msg_type == "bomb" {
                    self.server.lock().unwrap().put_bomb(id);
                } else if msg_type == "move" {
                    let msg: MoveMsg = Deserialize::deserialize(&mut de).unwrap_or(MoveMsg::new(Direction::North));
                    self.server.lock().unwrap().move_player(id, msg.direction);
                }
            }
        }
    }

    pub fn get_events(&mut self, id: u64) -> Vec<Vec<u8>> {
        self.server.lock().unwrap().get_events(&id)
    }

    /**
     * Process a stream (rx and tx datas)
     * @param id    The stream id
     * @return if the operation was successful
     */
    pub fn process_stream(&mut self, id: u64, buf: &Vec<u8>) {
        let mut pkts: Vec<Vec<u8>> = Vec::new();
        let rtp_buf = &mut self.streams[id as usize].rtp_buf;
        let size = buf.len() as u16;
        let mut parsed = 0;
        loop {
            let mut pkt_len = size - parsed;
            let mut store_remaining = true;
            let mut start = parsed;

            if rtp_buf.size != 0 || rtp_buf.wanted != 0 {
                // There is a packet to complete
                if rtp_buf.size == 1 {
                    pkt_len = ((rtp_buf.data[0] as u16) << 8) + buf[0] as u16;
                    rtp_buf.size = 0; // The packet is eaten
                    parsed += 1;
                    start += 1;
                    if pkt_len + parsed <= size {
                        store_remaining = false;
                        parsed += size;
                    } else {
                        rtp_buf.wanted = pkt_len;
                    }
                } else if pkt_len + rtp_buf.size >= rtp_buf.wanted {
                    // We have enough data to build the new packet to parse
                    store_remaining = false;
                    let eaten_bytes = rtp_buf.wanted - rtp_buf.size;
                    rtp_buf.data[rtp_buf.size as usize..]
                        .copy_from_slice(&buf[(parsed as usize)..(parsed as usize + eaten_bytes as usize)]);
                    pkt_len = rtp_buf.wanted;
                    parsed += eaten_bytes;
                    rtp_buf.size = 0;
                    rtp_buf.wanted = 0;
                }
            } else if pkt_len > 1 {
                pkt_len = ((buf[0] as u16) << 8) + buf[1] as u16;
                parsed += 2;
                start += 2;
                if pkt_len + parsed <= size {
                    store_remaining = false;
                    parsed += pkt_len;
                } else {
                    rtp_buf.wanted = pkt_len;
                }
            }
            if store_remaining {
                let stored_size = size - parsed;
                rtp_buf.data[rtp_buf.size as usize..]
                    .copy_from_slice(&buf[(parsed as usize)..(parsed as usize + stored_size as usize)]);
                rtp_buf.size += stored_size;
                break;
            }

            let pkt = buf[(start as usize)..(start as usize + pkt_len as usize)].to_vec();
            pkts.push(pkt);

            if parsed >= size {
                break;
            }
        }

        // Execute packts
        for pkt in pkts {
            self.parse_pkt(pkt, id);
        }
    }
}