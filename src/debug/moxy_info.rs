use tui::text::{Span, Spans};

pub struct MoxyInfo {
    pub method: String,
    pub path: String,
    pub mode: String,
    pub name: String,
    pub matching_rules: usize,
    pub response_code: String,
}

impl<'a> From<&MoxyInfo> for Spans<'a> {
    fn from(moxy_info: &MoxyInfo) -> Spans<'a> {
        Spans::from(vec![
            Span::from(moxy_info.method.to_owned()),
            Span::from(" Mode: "),
            Span::from(moxy_info.mode.to_owned()),
            Span::from(" => "),
            Span::from(moxy_info.response_code.to_owned()),
            Span::from(" Matched Rules: "),
            Span::from(moxy_info.matching_rules.to_owned().to_string()),
            Span::from(" Name: "),
            Span::from(moxy_info.name.to_owned()),
            Span::from(" -- "),
            Span::from(moxy_info.path.to_owned()),
        ])
    }
}
