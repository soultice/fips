use super::fips_info::FipsInfo;

#[derive(Clone)]
pub enum PrintInfo {
    Plain(String),
    #[allow(dead_code)]
    Fips(FipsInfo),
}
