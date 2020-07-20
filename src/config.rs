use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IrcConfig {
    pub user_data: UserData,
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
    pub hostname: String,
    pub port: u16,
    pub password: String,
    pub use_tls: bool,
    pub sasl: SaslConfig,
    pub nickserv: NickServConfig,
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
