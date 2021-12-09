use super::pimps_info::PimpsInfo;

pub enum PrintInfo {
    PLAIN(String),
    PIMPS(PimpsInfo),
}
