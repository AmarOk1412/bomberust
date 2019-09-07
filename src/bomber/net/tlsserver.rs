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

use std::fs::File;
use std::io::BufReader;
use std::net::ToSocketAddrs;
use tokio::net::TcpListener;
use tokio::prelude::{Future, Stream};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        Certificate, NoClientAuth, PrivateKey, ServerConfig,
        internal::pemfile::{ certs, rsa_private_keys }
    },
};
use std::sync::{Arc, Mutex};
use std::{thread, time};

use super::playerstreammanager::PlayerStreamManager;

/**
 * Server config
 */
pub struct TlsServerConfig {
    pub host: String,
    pub port: u16,
    pub cert: String,
    pub key: String,
    pub streams_manager: Arc<Mutex<PlayerStreamManager>>
}

fn load_certs(path: &str) -> Vec<Certificate> {
    certs(&mut BufReader::new(File::open(path).unwrap())).unwrap()
}

fn load_keys(path: &str) -> Vec<PrivateKey> {
    rsa_private_keys(&mut BufReader::new(File::open(path).unwrap())).unwrap()
}

/**
 * Listen for incoming connections and pass it to a PlayerStreamManager
 */
pub struct TlsServer {
}

impl TlsServer {
    pub fn start(config: &TlsServerConfig) {
        let addr = (&*config.host, config.port).to_socket_addrs().unwrap().next().unwrap();
        let mut server_config = ServerConfig::new(NoClientAuth::new());
        server_config.set_single_cert(
                load_certs(&*config.cert),
                load_keys(&*config.key).remove(0)
            ).expect("invalid key or certificate");
        let acceptor = TlsAcceptor::from(Arc::new(server_config));
        let socket = TcpListener::bind(&addr).unwrap();
        let stm = config.streams_manager.clone();
        let done = socket.incoming()
            .for_each(move |stream| {
                let addr = stream.peer_addr().ok();
                let stm = stm.clone();
                let done = acceptor.accept(stream)
                .and_then(move |stream| {
                    let id = stm.lock().unwrap().add_stream(stream);
                    let process_delay = time::Duration::from_nanos(100);
                    loop {
                        // This is done here to be in a tokio Task
                        if !stm.lock().unwrap().process_stream(id) {
                            break;
                        }
                        thread::sleep(process_delay);
                    }
                    Ok(id)
                })
                .map(move |id| warn!("TODO: {:?} quit", id))
                .map_err(move |err| error!("Error: {:?} - {:?}", err, addr));
                tokio::spawn(done);

                Ok(())
            });

        tokio::run(done.map_err(drop));
    }
}