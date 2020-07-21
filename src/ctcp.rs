use simple_irc::Prefix;

use crate::config::Server;
use crate::irc_state::IrcState;
use std::time::SystemTime;
use chrono::{DateTime, Utc};

pub struct CtcpRequest<'a> {
    pub server: &'a Server,
    pub irc_state: &'a IrcState,
    pub user: &'a Prefix,
    pub source: &'a String,
    pub command: &'a String,
    pub message: &'a String,
}

pub struct CtcpResponse {
    pub target: String,
    pub message: String,
}

pub trait CtcpEvent: Send + Sync {
    fn execute(&self, request: CtcpRequest) -> Option<CtcpResponse>;
}

pub struct VersionCtcpResponse {}

pub struct PingCtcpResponse {}

pub struct ClientInfoCtcpResponse {
    pub available_ctcp: Vec<String>,
}

pub struct FingerCtcpResponse {}

pub struct SourceCtcpResponse {}

pub struct TimeCtcpResponse {}

pub struct UserInfoCtcpResponse {}

impl CtcpEvent for VersionCtcpResponse {
    fn execute(&self, request: CtcpRequest) -> Option<CtcpResponse> {
        if request.command.eq("VERSION") {
            return Some(CtcpResponse {
                target: request.source.clone(),
                message: format!("VERSION {}", request.server.ctcp.version),
            });
        }

        None
    }
}

impl CtcpEvent for PingCtcpResponse {
    fn execute(&self, request: CtcpRequest) -> Option<CtcpResponse> {
        if request.command.eq("PING") {
            return Some(CtcpResponse {
                target: request.source.clone(),
                message: format!("PING {}", request.message),
            });
        }

        None
    }
}

impl CtcpEvent for ClientInfoCtcpResponse {
    fn execute(&self, request: CtcpRequest) -> Option<CtcpResponse> {
        if request.command.eq("CLIENTINFO") {
            return Some(CtcpResponse {
                target: request.source.clone(),
                message: format!("CLIENTINFO {}", self.available_ctcp.join(" ")),
            });
        }

        None
    }
}

impl CtcpEvent for FingerCtcpResponse {
    fn execute(&self, request: CtcpRequest) -> Option<CtcpResponse> {
        if request.command.eq("FINGER") {
            return Some(CtcpResponse {
                target: request.source.clone(),
                message: format!("FINGER {}", request.server.ctcp.version),
            });
        }

        None
    }
}

impl CtcpEvent for SourceCtcpResponse {
    fn execute(&self, request: CtcpRequest) -> Option<CtcpResponse> {
        if request.command.eq("SOURCE") {
            return Some(CtcpResponse {
                target: request.source.clone(),
                message: format!("SOURCE {}", request.server.ctcp.source),
            });
        }

        None
    }
}

impl CtcpEvent for TimeCtcpResponse {
    fn execute(&self, request: CtcpRequest) -> Option<CtcpResponse> {
        if request.command.eq("TIME") {
            let datetime: DateTime<Utc> = SystemTime::now().into();

            return Some(CtcpResponse {
                target: request.source.clone(),
                message: format!("TIME {}", datetime.format("%c")),
            });
        }

        None
    }
}

impl CtcpEvent for UserInfoCtcpResponse {
    fn execute(&self, request: CtcpRequest) -> Option<CtcpResponse> {
        if request.command.eq("USERINFO") {
            return Some(CtcpResponse {
                target: request.source.clone(),
                message: format!("USERINFO {} ({})", request.server.user_data.nickname, request.server.user_data.realname),
            });
        }

        None
    }
}