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

extern crate clap;
extern crate rand;
extern crate tokio;
extern crate tokio_rustls;

pub mod bomberust;

use bomberust::game::Game;

use std::fs::File;
use std::io::{BufReader, Write};
use std::net::ToSocketAddrs;
use tokio::io::{ self, AsyncRead };
use tokio::net::TcpListener;
use tokio::prelude::{Future, Stream};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        Certificate, NoClientAuth, PrivateKey, ServerConfig,
        internal::pemfile::{ certs, rsa_private_keys }
    },
};
use std::sync::Arc;
use std::thread;

fn load_certs(path: &str) -> Vec<Certificate> {
    certs(&mut BufReader::new(File::open(path).unwrap())).unwrap()
}

fn load_keys(path: &str) -> Vec<PrivateKey> {
    rsa_private_keys(&mut BufReader::new(File::open(path).unwrap())).unwrap()
}

fn main() {
    let server_thread = thread::spawn(move || {
        let addr = "0.0.0.0:2542".to_socket_addrs().unwrap().next().unwrap();

        let mut config = ServerConfig::new(NoClientAuth::new());
        config.set_single_cert(load_certs("./keys/ca/rsa/end.fullchain"), load_keys("./keys/ca/rsa/end.rsa").remove(0))
            .expect("invalid key or certificate");
        let config = TlsAcceptor::from(Arc::new(config));

        let socket = TcpListener::bind(&addr).unwrap();
        let done = socket.incoming()
            .for_each(move |stream| {
                let addr = stream.peer_addr().ok();
            let done = config.accept(stream)
                .and_then(|stream| {
                    let (reader, writer) = stream.split();
                    io::copy(reader, writer)
                    //io::write_all(stream, &b"HELLO FROM SERVER"[..])
                })
                .map(move |(n, ..)| println!("Echo: {} - {:?}", n, addr))
                //.and_then(|(stream, _)| io::flush(stream))
                //.map(move |_| println!("Accept: {:?}", addr))
                .map_err(move |err| println!("Error: {:?} - {:?}", err, addr));
                tokio::spawn(done);

                Ok(())
            });

        tokio::run(done.map_err(drop));
    });

    let mut g = Game::new();
    g.start();
    let _ = server_thread.join();
}
