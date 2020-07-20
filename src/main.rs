use std::net::{TcpStream, ToSocketAddrs};

use anyhow::Result;
use async_dup::Mutex;
use async_io::Async;
use blocking::block_on;
use futures::io::BufReader;
use futures::prelude::*;
use simple_irc::Message;

fn main() -> Result<()> {
    block_on(async {
        for socket_addr in "irc.rizon.net:6697".to_socket_addrs()? {
            let stream_result = Async::<TcpStream>::connect(socket_addr).await;

            match stream_result {
                Ok(stream) => {
                    let stream = async_native_tls::connect("irc.rizon.net", stream).await?;

                    println!("Connected to {}", stream.get_ref().get_ref().peer_addr()?);

                    let mut stream = &Mutex::new(stream);
                    let mut lines_from_server = BufReader::new(stream).lines();

                    let mut sent_user = false;

                    while let Some(line) = lines_from_server.next().await {
                        let line = line?;
                        let message = line.parse::<simple_irc::Message>()?;

                        println!("{}", line);

                        if !sent_user {
                            sent_user = true;

                            write_message(&Message::new("NICK".to_string(), vec![
                                "hfinch".to_string()
                            ]), &mut stream).await?;

                            write_message(&Message::new("USER".to_string(), vec![
                                "hfinch".to_string(),
                                "0".to_string(),
                                "*".to_string(),
                                "hfinch".to_string()
                            ]), &mut stream).await?;
                        }

                        match message.command.as_str() {
                            "PING" => {
                                write_message(&Message::new("PONG".to_string(), message.params), &mut stream).await?;
                            }
                            _ => {
                                println!("Unknown command. {}", message.command)
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("{:}", e);
                    continue;
                }
            }
        }

        Ok(())
    })
}

async fn write_message(message: &Message, writer: &mut (impl AsyncWrite + Unpin)) -> Result<()> {
    let message = message.to_string();

    println!("{}", message);

    writer.write_all((message + "\r\n").as_bytes()).await?;

    return Ok(());
}