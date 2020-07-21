use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IrcConfig {
    pub servers: Vec<Server>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub nickname: String,
    pub username: String,
    pub realname: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Server {
    pub user_data: UserData,
    pub hostname: String,
    pub port: u16,
    pub password: String,
    pub use_tls: bool,
    pub use_hostserv: bool,
    pub sasl: SaslConfig,
    pub nickserv: NickServConfig,
    pub ctcp: CtcpConfig,
    #[serde(default)]
    pub channels: Vec<ChannelConfig>,
    pub privmsg_plugins: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaslConfig {
    pub enabled: bool,
    pub user: String,
    pub password: String,
    pub terminate_failed: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NickServConfig {
    pub enabled: bool,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CtcpConfig {
    pub enabled: Vec<String>,
    pub version: String,
    pub source: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChannelConfig {
    pub name: String,
    pub password: String,
}
