use gradient_tui_fork::text::{Span, Spans};

#[derive(Clone)]
pub struct FipsInfo {
    pub method: String,
    pub path: String,
    pub mode: String,
    pub name: String,
    pub matching_rules: usize,
    pub response_code: String,
}

impl<'a> From<&FipsInfo> for Spans<'a> {
    fn from(fips_info: &FipsInfo) -> Spans<'a> {
        Spans::from(vec![
            Span::from(fips_info.method.to_owned()),
            Span::from(" Mode: "),
            Span::from(fips_info.mode.to_owned()),
            Span::from(" => "),
            Span::from(fips_info.response_code.to_owned()),
            Span::from(" Matched Rules: "),
            Span::from(fips_info.matching_rules.to_owned().to_string()),
            Span::from(" Name: "),
            Span::from(fips_info.name.to_owned()),
            Span::from(" -- "),
            Span::from(fips_info.path.to_owned()),
        ])
    }
}
