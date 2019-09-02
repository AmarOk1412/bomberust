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

use futures::Async;
use tokio_rustls::server::TlsStream;
use tokio::net::TcpStream;
use tokio::io::AsyncRead;
use std::io::Write;

pub struct Stream {
    id: u64,
    stream: TlsStream<TcpStream>,
}

pub struct PlayerStreamManager {
    current_id: u64,
    pub streams: Vec<Stream>
}


impl PlayerStreamManager {
    pub fn new() -> PlayerStreamManager {
        PlayerStreamManager {
            current_id: 0,
            streams: Vec::new()
        }
    }

    pub fn add_stream(&mut self, stream: TlsStream<TcpStream>) -> u64 {
        let id = self.current_id;
        self.streams.push(Stream {
            id,
            stream,
        });
        self.current_id += 1;
        id
    }

    pub fn process_stream(&mut self, id: u64) -> bool {
        let mut buf = [0; 1024];
        let mut result = true;
        match self.streams[id as usize].stream.poll_read(&mut buf) {
            Ok(Async::Ready(n)) => {
                result = n != 0;
            },
            Ok(Async::NotReady) => {}
            Err(_) => { result = false; }
        };
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

    pub fn on_rx(&mut self, id: &u64, data: &String) {
        println!("Client {}, said: {}", id, data);
    }
}