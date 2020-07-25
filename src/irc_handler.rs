use futures::io::BufReader;
use futures::prelude::*;
use simple_irc::Message;

use crate::config::Server;
use crate::ctcp::{CtcpEvent, CtcpRequest};
use crate::irc_ext::IrcExt;
use crate::irc_state::IrcState;
use crate::privmsg::{PrivMsgEvent, PrivMsgRequest};

pub struct IrcHandler<'a> {
    pub server: &'a mut Server,
    pub irc_state: &'a mut IrcState,
    pub ctcp_event: &'a Vec<Box<dyn CtcpEvent>>,
    pub privmsg_event: &'a Vec<Box<dyn PrivMsgEvent>>,
}

impl IrcHandler<'_> {
    pub async fn handle(&mut self, reader: impl AsyncRead + Unpin, writer: &mut (impl AsyncWrite + Unpin)) {
        let mut lines_from_server = BufReader::new(reader).lines();

        if self.irc_state.initial_connection {
            self.handle_initial_connection(writer).await;
        }

        while let Some(line) = lines_from_server.next().await {
            match line {
                Ok(line) => {
                    let message = &line.parse::<simple_irc::Message>().unwrap();

                    log::debug!("{}", message);

                    match message.command.as_str() {
                        "CAP" => self.handle_cap(message, writer).await,
                        "AUTHENTICATE" => self.handle_authenticate(message, writer).await,
                        "904" => self.handle_authenticate_fail(writer).await,
                        "900" => (),
                        "903" => self.handle_authenticate_success(writer).await,
                        "NOTICE" => (),
                        "001" => (),
                        "002" => (),
                        "003" => (),
                        "004" => (),
                        "005" => (),
                        "251" => (),
                        "252" => (),
                        "253" => (),
                        "254" => (),
                        "255" => (),
                        "265" => (),
                        "266" => (),
                        "375" => (),
                        "372" => (),
                        "JOIN" => (),
                        "353" => (),
                        "366" => (),
                        "333" => (),
                        "332" => (),
                        "354" => (),
                        "315" => (),
                        "376" => self.handle_end_motd(message, writer).await,
                        "MODE" => self.handle_mode(message, writer).await,
                        "PRIVMSG" => self.handle_privmsg(message, writer).await,
                        "PING" => self.handle_ping(message, writer).await,
                        _ => {
                            log::warn!("Unknown command. {}", message.command)
                        }
                    }
                },
                Err(e) => log::error!("{}", e),
            }
        }
    }

    async fn handle_initial_connection(&mut self, writer: &mut (impl AsyncWrite + Unpin)) {
        // The recommended order of commands during registration is as follows:
        // CAP LS 302
        // PASS
        // NICK and USER
        // Capability Negotiation
        // SASL (if negotiated)
        // CAP END

        self.write_message(&Message::new("CAP".to_string(), vec![
            "LS".to_string(),
            "302".to_string(),
        ]), writer).await;

        if !self.server.password.is_empty() {
            self.write_message(&Message::new("PASS".to_string(), vec![
                self.server.password.clone(),
            ]), writer).await;
        }

        self.write_message(&Message::new("NICK".to_string(), vec![
            self.server.user_data.nickname.clone()
        ]), writer).await;

        self.write_message(&Message::new("USER".to_string(), vec![
            self.server.user_data.nickname.clone(),
            "0".to_string(),
            "*".to_string(),
            self.server.user_data.realname.clone()
        ]), writer).await;

        self.irc_state.initial_connection = false;
        self.irc_state.negotiating_cap = true;
    }

    async fn handle_cap(&mut self, message: &Message, writer: &mut (impl AsyncWrite + Unpin)) {
        if self.irc_state.negotiating_cap {
            let cap_type = message.params[1].as_str();

            match cap_type {
                "LS" => {
                    let available_caps: Vec<&str> = message.params[2].split(" ").collect();

                    log::debug!("Available CAP: {:?}", available_caps);

                    for requested_cap in &self.irc_state.cap_requested {
                        if available_caps.contains(&requested_cap.as_str()) {
                            self.irc_state.cap_negotiated.push(requested_cap.clone());
                        } else {
                            log::warn!("No available CAP: {}", requested_cap);
                        }
                    }

                    self.write_message(&Message::new("CAP".to_string(), vec![
                        "REQ".to_string(),
                        self.irc_state.cap_negotiated.join(" "),
                    ]), writer).await;
                }
                "ACK" => {
                    let cap_accepted: Vec<&str> = message.params[2].split(" ").collect();

                    for cap in cap_accepted {
                        self.irc_state.cap_accepted.push(cap.to_string());
                    }

                    if self.server.sasl.enabled && self.irc_state.cap_accepted.contains(&"sasl".to_string()) {
                        self.irc_state.negotiating_sasl = true;

                        self.write_message(&Message::new("AUTHENTICATE".to_string(), vec![
                            "PLAIN".to_string(),
                        ]), writer).await;
                    }

                    if self.irc_state.cap_negotiated.len() == self.irc_state.cap_accepted.len() {
                        self.irc_state.negotiating_cap = false;

                        if !self.irc_state.negotiating_sasl {
                            // Finish CAP negotiation
                            self.finish_cap(writer).await;
                        }
                    }
                }
                _ => {
                    log::warn!("Unknown CAP type: {}", cap_type);
                }
            }
        }
    }

    async fn handle_authenticate(&mut self, message: &Message, writer: &mut (impl AsyncWrite + Unpin)) {
        if self.irc_state.negotiating_sasl {
            let authenticate_type = message.params[0].as_str();

            match authenticate_type {
                "+" => {
                    let encoded = base64::encode(format!("{}\0{}\0{}", self.server.sasl.user, self.server.sasl.user, self.server.sasl.password));

                    self.write_message(&Message::new("AUTHENTICATE".to_string(), vec![
                        encoded,
                    ]), writer).await;
                }
                _ => {
                    log::warn!("Unknown AUTHENTICATE type: {}", authenticate_type);
                }
            }
        }
    }

    async fn handle_authenticate_fail(&mut self, writer: &mut (impl AsyncWrite + Unpin)) {
        if self.irc_state.negotiating_sasl {
            log::error!("SASL Authentication failed");

            self.irc_state.negotiating_sasl = false;

            if self.server.sasl.terminate_failed {
                self.write_message(&Message::new("QUIT".to_string(), vec![
                    "SASL Authentication failed".to_string(),
                ]), writer).await;
            }

            self.finish_cap(writer).await;
        }
    }

    async fn handle_authenticate_success(&mut self, writer: &mut (impl AsyncWrite + Unpin)) {
        if self.irc_state.negotiating_sasl {
            log::info!("SASL Authentication success");

            self.irc_state.negotiating_sasl = false;

            self.finish_cap(writer).await;
        }
    }

    async fn finish_cap(&mut self, writer: &mut (impl AsyncWrite + Unpin)) {
        self.write_message(&Message::new("CAP".to_string(), vec![
            "END".to_string(),
        ]), writer).await;
    }

    async fn handle_privmsg(&mut self, message: &Message, writer: &mut (impl AsyncWrite + Unpin)) {
        let mut source = &message.params[0];
        let msg = &message.params[1];

        if !source.is_channel_name() {
            source = &message.prefix.as_ref().unwrap().nick;
        }

        if msg.is_ctcp() {
            let msg = msg.replace("\u{1}", "");
            let command: &str = msg.split(" ").next().unwrap();
            let msg = msg[command.len()..].trim();

            for event in self.ctcp_event {
                if let Some(response) = event.execute(CtcpRequest {
                    server: self.server,
                    irc_state: self.irc_state,
                    user: message.prefix.as_ref().unwrap(),
                    source,
                    command: &command.to_string(),
                    message: &msg.to_string(),
                }) {
                    self.send_notice(response.target, format!("\u{1}{}\u{1}", response.message), writer).await;

                    break;
                }
            }
        } else {
            for event in self.privmsg_event {
                if let Some(response) = event.execute(PrivMsgRequest {
                    server: self.server,
                    irc_state: self.irc_state,
                    user: message.prefix.as_ref().unwrap(),
                    source,
                    message: msg,
                }) {
                    self.send_privmsg(response.target, response.message, writer).await;
                }
            }
        }
    }

    async fn handle_end_motd(&mut self, _message: &Message, _writer: &mut (impl AsyncWrite + Unpin)) {}

    async fn handle_mode(&mut self, message: &Message, writer: &mut (impl AsyncWrite + Unpin)) {
        let _user = &message.params[0];
        let mode_type = &message.params[1][..1];
        let mode: &Vec<char> = &message.params[1][1..].chars().collect();

        if mode_type == "+" && mode.contains(&'r') {
            if self.server.use_hostserv {
                self.send_privmsg("HostServ".to_string(), "ON".to_string(), writer).await;
            }

            // Nick is registered, join channels defined in server config
            for channel in &self.server.channels {
                self.write_message(&Message::new("JOIN".to_string(), vec![
                    channel.name.clone(),
                    channel.password.clone(),
                ]), writer).await;
            }
        }
    }

    async fn handle_ping(&mut self, message: &Message, writer: &mut (impl AsyncWrite + Unpin)) {
        self.write_message(&Message::new("PONG".to_string(), message.params.clone()), writer).await;
    }

    async fn send_privmsg(&self, target: String, message: String, writer: &mut (impl AsyncWrite + Unpin)) {
        self.write_message(&Message::new("PRIVMSG".to_string(), vec![
            target,
            message,
        ]), writer).await;
    }

    async fn send_notice(&self, target: String, message: String, writer: &mut (impl AsyncWrite + Unpin)) {
        self.write_message(&Message::new("NOTICE".to_string(), vec![
            target,
            message,
        ]), writer).await;
    }

    async fn write_message(&self, message: &Message, writer: &mut (impl AsyncWrite + Unpin)) {
        let message = message.to_string();

        log::debug!("{}", message);

        writer.write_all((message + "\r\n").as_bytes()).await.unwrap();
    }
}
