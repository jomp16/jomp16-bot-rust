use regex::Regex;

pub trait IrcExt {
    fn is_channel_name(&self) -> bool;

    fn is_ctcp(&self) -> bool;

    fn remove_colorization(&self) -> String;
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

    fn remove_colorization(&self) -> String {
        // https://stackoverflow.com/a/3504063
        let re = Regex::new(r"\x1f|\x02|\x12|\x0f|\x16|\x03(?:\d{1,2}(?:,\d{1,2})?)?").unwrap();
        let result = re.replace_all(self, "").to_string();

        return result;
    }
}

impl IrcExt for String {
    fn is_channel_name(&self) -> bool {
        return (&self[..]).is_channel_name();
    }

    fn is_ctcp(&self) -> bool {
        return (&self[..]).is_ctcp();
    }

    fn remove_colorization(&self) -> String {
        return (&self[..]).remove_colorization();
    }
}