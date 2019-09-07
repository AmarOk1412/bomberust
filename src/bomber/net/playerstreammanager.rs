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

use super::super::core::Server;
use super::super::net::msg::*;

use futures::Async;
use tokio_rustls::server::TlsStream;
use tokio::net::TcpStream;
use tokio::io::AsyncRead;
use rmps::Deserializer;
use rmps::decode::Error;
use serde::Deserialize;
use std::io::{Cursor, Write};
use std::sync::{Arc, Mutex};

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
    stream: TlsStream<TcpStream>,
    rtp_buf: RtpBuf,
}

/**
 * Manager incoming streams and pass events to the Server 
 */
pub struct PlayerStreamManager {
    current_id: u64,
    pub streams: Vec<Stream>,
    pub server: Arc<Mutex<Server>>
}


impl PlayerStreamManager {
    /**
     * Generate a new PlayerStreamManager
     */
    pub fn new(server: Arc<Mutex<Server>>) -> PlayerStreamManager {
        PlayerStreamManager {
            current_id: 0,
            streams: Vec::new(),
            server
        }
    }

    /**
     * Add a stream to process
     * @param stream    The stream to add
     * @return          The stream id
     */
    pub fn add_stream(&mut self, stream: TlsStream<TcpStream>) -> u64 {
        let id = self.current_id;
        self.streams.push(Stream {
            id,
            stream,
            rtp_buf: RtpBuf {
                data: [0; 65536],
                size: 0,
                wanted: 0,
            } 
        });
        self.current_id += 1;
        self.server.lock().unwrap().join_server(id);
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
                self.server.lock().unwrap().create_room(id);
            } else if msg_type == "join" {
                let msg: JoinMsg = Deserialize::deserialize(&mut de).unwrap_or(JoinMsg::new(0));
                self.server.lock().unwrap().join_room(id, msg.room);
            } else if msg_type == "launch" {
                self.server.lock().unwrap().launch_game(id);
            } else if msg_type == "bomb" {
                self.server.lock().unwrap().put_bomb(id);
            }
        }
    }

    /**
     * Process a stream (rx and tx datas)
     * @param id    The stream id
     * @return if the operation was successful
     */
    pub fn process_stream(&mut self, id: u64) -> bool {
        let mut buf = [0; 1024];
        let mut result = true;
        let stream = &mut self.streams[id as usize];
        let rtp_buf = &mut stream.rtp_buf;
        let socket = &mut stream.stream;
        let mut pkts: Vec<Vec<u8>> = Vec::new();
        match socket.poll_read(&mut buf) {
            Ok(Async::Ready(n)) => {
                result = n != 0;
                let size = n as u16;
                if result {
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
                }
            },
            Ok(Async::NotReady) => {}
            Err(_) => { result = false; }
        };
        // Execute packts
        for pkt in pkts {
            self.parse_pkt(pkt, id);
        }
        if !result {
            return false;
        }
        /* TODO write from server
        match self.streams[id as usize].stream.write(String::from("HELLO\n").as_bytes()) {
            Err(_) => {
                result = false;
            }
            _ => {}
        }*/
        result
    }
}