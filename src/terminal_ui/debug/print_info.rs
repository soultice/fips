use super::fips_info::FipsInfo;

#[derive(Clone)]
pub enum PrintInfo {
    PLAIN(String),
    FIPS(FipsInfo),
}
