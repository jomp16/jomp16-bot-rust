extern crate pretty_env_logger;

use std::env;
use std::fs::File;
use std::net::{TcpStream, ToSocketAddrs};

use anyhow::{anyhow, Result};
use async_dup::Mutex;
use async_io::Async;
use smol::Task;

use crate::config::IrcConfig;
use crate::ctcp::{ClientInfoCtcpResponse, CtcpEvent, FingerCtcpResponse, PingCtcpResponse, SourceCtcpResponse, TimeCtcpResponse, UserInfoCtcpResponse, VersionCtcpResponse};
use crate::irc_handler::IrcHandler;
use crate::irc_state::IrcState;
use crate::privmsg::{GeoIpPrivMsgEvent, PrivMsgEvent};

mod ctcp;
mod irc_ext;
mod geoip_response;
mod privmsg;
mod irc_handler;
mod irc_state;
mod config;

fn main() -> Result<()> {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "jomp16_bot_own=debug");
    }

    pretty_env_logger::init();

    let config: IrcConfig = serde_yaml::from_reader(File::open("config.yml")?)?;

    if config.servers.len() == 0 {
        return Err(anyhow!("No servers!"));
    }

    let mut futures = vec![];

    for server in config.servers {
        futures.push(Task::spawn(async move {
            for socket_addr in format!("{}:{}", &server.hostname, server.port).to_socket_addrs().unwrap() {
                let stream_result = Async::<TcpStream>::connect(socket_addr).await;

                match stream_result {
                    Ok(stream) => {
                        let irc_state = &mut IrcState { ..Default::default() };

                        if server.sasl.enabled {
                            irc_state.cap_requested.push("sasl".to_string());
                        }

                        let mut privmsg_plugins: Vec<Box<dyn PrivMsgEvent>> = vec![];
                        let mut ctcp_plugins: Vec<Box<dyn CtcpEvent>> = vec![];

                        for plugin in &server.privmsg_plugins {
                            match plugin.as_str() {
                                "geoip" => privmsg_plugins.push(Box::new(GeoIpPrivMsgEvent { ..Default::default() })),
                                _ => log::warn!("Unknown plugin: {}", plugin),
                            }
                        }

                        for plugin in &server.ctcp.enabled {
                            match plugin.as_str() {
                                "CLIENTINFO" => ctcp_plugins.push(Box::new(ClientInfoCtcpResponse { available_ctcp: server.ctcp.enabled.clone() })),
                                "FINGER" => ctcp_plugins.push(Box::new(FingerCtcpResponse {})),
                                "PING" => ctcp_plugins.push(Box::new(PingCtcpResponse {})),
                                "SOURCE" => ctcp_plugins.push(Box::new(SourceCtcpResponse {})),
                                "TIME" => ctcp_plugins.push(Box::new(TimeCtcpResponse {})),
                                "VERSION" => ctcp_plugins.push(Box::new(VersionCtcpResponse {})),
                                "USERINFO" => ctcp_plugins.push(Box::new(UserInfoCtcpResponse {})),
                                _ => log::warn!("Unknown CTCP plugin: {}", plugin),
                            }
                        }

                        let mut handler = IrcHandler {
                            server: &mut server.clone(),
                            irc_state,
                            ctcp_event: &ctcp_plugins,
                            privmsg_event: &privmsg_plugins,
                        };

                        if server.use_tls {
                            let stream = async_native_tls::connect(&server.hostname, stream).await.unwrap();
                            let mut stream = &Mutex::new(stream);

                            handler.handle(stream, &mut stream).await;
                        } else {
                            let mut stream = &Mutex::new(stream);

                            handler.handle(stream, &mut stream).await;
                        }
                    }
                    Err(e) => {
                        log::error!("{:}", e);
                        continue;
                    }
                }
            }
        }));
    }

    smol::run(async {
        futures::future::join_all(futures).await;

        Ok(())
    })
}

