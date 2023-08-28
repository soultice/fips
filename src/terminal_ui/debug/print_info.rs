use super::fips_info::FipsInfo;

#[derive(Clone)]
pub enum PrintInfo {
    Plain(String),
    Fips(FipsInfo),
}
