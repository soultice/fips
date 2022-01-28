use super::fips_info::FipsInfo;

pub enum PrintInfo {
    PLAIN(String),
    FIPS(FipsInfo),
}
