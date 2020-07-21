pub trait IrcExt {
    fn is_channel_name(&self) -> bool;

    fn is_ctcp(&self) -> bool;
}

impl<'a> IrcExt for &'a str {
    fn is_channel_name(&self) -> bool {
        return self.starts_with('#')
            || self.starts_with('&')
            || self.starts_with('+')
            || self.starts_with('!');
    }

    fn is_ctcp(&self) -> bool {
        return self.starts_with('\u{1}');
    }
}

impl IrcExt for String {
    fn is_channel_name(&self) -> bool {
        return (&self[..]).is_channel_name();
    }

    fn is_ctcp(&self) -> bool {
        return (&self[..]).is_ctcp();
    }
}