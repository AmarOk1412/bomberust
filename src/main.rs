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

extern crate env_logger;
#[macro_use]
extern crate log;
extern crate futures;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate typetag;
extern crate rmp_serde as rmps;
extern crate tokio;
extern crate tokio_rustls;

pub mod bomber;

use bomber::core::Server;
use bomber::net::{PlayerStreamManager, TlsServer, TlsServerConfig};

use std::sync::{Arc, Mutex};
use std::thread;


fn main() {
    // Init logging
    env_logger::init();

    let server = Arc::new(Mutex::new(Server::new()));
    let streams_manager = Arc::new(Mutex::new(PlayerStreamManager::new(server)));
    let server_thread = thread::spawn(move || {
        // TODO get config from file
        let config = TlsServerConfig {
            host : String::from("0.0.0.0"),
            port : 2542,
            cert : String::from("./keys/ca/rsa/end.fullchain"),
            key : String::from("./keys/ca/rsa/end.rsa"),
            streams_manager: streams_manager,
        };
        TlsServer::start(&config);
    });

    let _ = server_thread.join();
}
