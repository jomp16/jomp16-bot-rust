use anyhow::{anyhow, Result};
use futures::AsyncWrite;
use futures::prelude::*;
use simple_irc::Message;

use crate::config::{Server, UserData};
use crate::irc_state::IrcState;

pub struct IrcHandler<'a> {
    pub user_data: &'a UserData,
    pub server: &'a mut Server,
    pub irc_state: &'a mut IrcState,
}

impl IrcHandler<'_> {
    pub async fn handle(&mut self, message: &Message, writer: &mut (impl AsyncWrite + Unpin)) -> Result<()> {
        log::debug!("{}", message);

        if self.irc_state.initial_connection {
            self.handle_initial_connection(writer).await?;
        }

        match message.command.as_str() {
            "CAP" => self.handle_cap(message, writer).await?,
            "AUTHENTICATE" => self.handle_authenticate(message, writer).await?,
            "904" => self.handle_authenticate_fail(writer).await?,
            "900" => (),
            "903" => self.handle_authenticate_success(writer).await?,
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
            "MODE" => (),
            "PRIVMSG" => (),
            "PING" => {
                self.write_message(&Message::new("PONG".to_string(), message.params.clone()), writer).await?;
            }
            _ => {
                log::warn!("Unknown command. {}", message.command)
            }
        }

        Ok(())
    }

    async fn handle_initial_connection(&mut self, writer: &mut (impl AsyncWrite + Unpin)) -> Result<()> {
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
        ]), writer).await?;

        if !self.server.password.is_empty() {
            self.write_message(&Message::new("PASS".to_string(), vec![
                self.server.password.clone(),
            ]), writer).await?;
        }

        self.write_message(&Message::new("NICK".to_string(), vec![
            self.user_data.nickname.clone()
        ]), writer).await?;

        self.write_message(&Message::new("USER".to_string(), vec![
            self.user_data.nickname.clone(),
            "0".to_string(),
            "*".to_string(),
            self.user_data.realname.clone()
        ]), writer).await?;

        self.irc_state.initial_connection = false;
        self.irc_state.negotiating_cap = true;

        Ok(())
    }

    async fn handle_cap(&mut self, message: &Message, writer: &mut (impl AsyncWrite + Unpin)) -> Result<()> {
        if self.irc_state.negotiating_cap {
            log::debug!("{:?}", message.params);

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
                    ]), writer).await?;
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
                        ]), writer).await?;
                    }

                    if self.irc_state.cap_negotiated.len() == self.irc_state.cap_accepted.len() {
                        self.irc_state.negotiating_cap = false;

                        if !self.irc_state.negotiating_sasl {
                            // Finish CAP negotiation
                            self.finish_cap(writer).await?;
                        }
                    }
                }
                _ => {
                    log::warn!("Unknown CAP type: {}", cap_type);
                }
            }
        }

        Ok(())
    }

    async fn handle_authenticate(&mut self, message: &Message, writer: &mut (impl AsyncWrite + Unpin)) -> Result<()> {
        if self.irc_state.negotiating_sasl {
            log::debug!("{:?}", message.params);

            let authenticate_type = message.params[0].as_str();

            match authenticate_type {
                "+" => {
                    let encoded = base64::encode(format!("{}\0{}\0{}", self.server.sasl.user, self.server.sasl.user, self.server.sasl.password));

                    self.write_message(&Message::new("AUTHENTICATE".to_string(), vec![
                        encoded,
                    ]), writer).await?;
                }
                _ => {
                    log::warn!("Unknown AUTHENTICATE type: {}", authenticate_type);
                }
            }
        }

        Ok(())
    }

    async fn handle_authenticate_fail(&mut self, writer: &mut (impl AsyncWrite + Unpin)) -> Result<()> {
        if self.irc_state.negotiating_sasl {
            log::error!("SASL Authentication failed");

            self.irc_state.negotiating_sasl = false;

            if self.server.sasl.terminate_failed {
                self.write_message(&Message::new("QUIT".to_string(), vec![
                    "SASL Authentication failed".to_string(),
                ]), writer).await?;

                return Err(anyhow!("SASL Authentication failed"));
            }

            self.finish_cap(writer).await?;
        }

        Ok(())
    }

    async fn handle_authenticate_success(&mut self, writer: &mut (impl AsyncWrite + Unpin)) -> Result<()> {
        if self.irc_state.negotiating_sasl {
            log::info!("SASL Authentication success");

            self.irc_state.negotiating_sasl = false;

            self.finish_cap(writer).await?;
        }

        Ok(())
    }

    async fn finish_cap(&mut self, writer: &mut (impl AsyncWrite + Unpin)) -> Result<()> {
        self.write_message(&Message::new("CAP".to_string(), vec![
            "END".to_string(),
        ]), writer).await?;

        Ok(())
    }

    async fn write_message(&self, message: &Message, writer: &mut (impl AsyncWrite + Unpin)) -> Result<()> {
        let message = message.to_string();

        log::debug!("{}", message);

        writer.write_all((message + "\r\n").as_bytes()).await?;

        return Ok(());
    }
}
