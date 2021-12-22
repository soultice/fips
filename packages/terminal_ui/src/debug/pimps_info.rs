use tui::text::{Span, Spans};

pub struct PimpsInfo {
    pub method: String,
    pub path: String,
    pub mode: String,
    pub name: String,
    pub matching_rules: usize,
    pub response_code: String,
}

impl<'a> From<&PimpsInfo> for Spans<'a> {
    fn from(pimps_info: &PimpsInfo) -> Spans<'a> {
        Spans::from(vec![
            Span::from(pimps_info.method.to_owned()),
            Span::from(" Mode: "),
            Span::from(pimps_info.mode.to_owned()),
            Span::from(" => "),
            Span::from(pimps_info.response_code.to_owned()),
            Span::from(" Matched Rules: "),
            Span::from(pimps_info.matching_rules.to_owned().to_string()),
            Span::from(" Name: "),
            Span::from(pimps_info.name.to_owned()),
            Span::from(" -- "),
            Span::from(pimps_info.path.to_owned()),
        ])
    }
}
