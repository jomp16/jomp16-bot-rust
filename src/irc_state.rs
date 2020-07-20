pub struct IrcState {
    pub initial_connection: bool,
    pub negotiating_cap: bool,
    pub negotiating_sasl: bool,
    pub sent_user: bool,
    pub cap_requested: Vec<String>,
    pub cap_negotiated: Vec<String>,
    pub cap_accepted: Vec<String>,
}

impl Default for IrcState {
    fn default() -> Self {
        IrcState {
            initial_connection: true,
            negotiating_cap: false,
            negotiating_sasl: false,
            sent_user: false,
            cap_requested: vec![
                "multi-prefix".to_string(), // https://ircv3.net/specs/extensions/multi-prefix-3.1.html
                "userhost-in-names".to_string(), // https://ircv3.net/specs/extensions/userhost-in-names-3.2
            ],
            cap_negotiated: vec![],
            cap_accepted: vec![],
        }
    }
}