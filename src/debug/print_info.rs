use super::moxy_info::MoxyInfo;

pub enum PrintInfo {
    PLAIN(String),
    MOXY(MoxyInfo),
}
