mod irc_handler;

extern crate pretty_env_logger;

use std::env;
use std::fs::File;
use std::net::{TcpStream, ToSocketAddrs};

use anyhow::{anyhow, Result};
use async_dup::Mutex;
use async_io::Async;
use blocking::block_on;
use futures::io::BufReader;
use futures::prelude::*;

use crate::config::IrcConfig;
use crate::irc_state::IrcState;
use crate::irc_handler::IrcHandler;

mod irc_state;
mod config;

fn main() -> Result<()> {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "debug");
    }

    pretty_env_logger::init();

    let config: &IrcConfig = &serde_yaml::from_reader(File::open("config.yml")?)?;

    if config.servers.len() == 0 {
        return Err(anyhow!("No servers!"));
    }

    block_on(async {
        let server = config.servers.first().unwrap();

        for socket_addr in format!("{}:{}", &server.hostname, server.port).to_socket_addrs()? {
            let stream_result = Async::<TcpStream>::connect(socket_addr).await;

            match stream_result {
                Ok(stream) => {
                    let irc_state = &mut IrcState { ..Default::default() };

                    if server.sasl.enabled {
                        irc_state.cap_requested.push("sasl".to_string());
                    }

                    let mut handler = IrcHandler {
                        user_data: &config.user_data,
                        server: &mut server.clone(),
                        irc_state,
                    };

                    let stream = async_native_tls::connect(&server.hostname, stream).await?;

                    log::info!("Connected to {}", stream.get_ref().get_ref().peer_addr()?);

                    let mut stream = &Mutex::new(stream);
                    let mut lines_from_server = BufReader::new(stream).lines();

                    while let Some(line) = lines_from_server.next().await {
                        let message = &line?.parse::<simple_irc::Message>()?;

                        handler.handle(message, &mut stream).await?;
                    }
                }
                Err(e) => {
                    log::error!("{:}", e);
                    continue;
                }
            }
        }

        Ok(())
    })
}

