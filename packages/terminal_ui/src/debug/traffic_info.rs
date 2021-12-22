use super::request_info::RequestInfo;
use super::response_info::ResponseInfo;
use std::fmt;
use tui::text::Text;

#[derive(Clone)]
pub enum TrafficInfo {
    IncomingRequest(RequestInfo),
    OutgoingRequest(RequestInfo),
    IncomingResponse(ResponseInfo),
    OutgoingResponse(ResponseInfo),
}

impl fmt::Display for TrafficInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let info_string = match &self {
            TrafficInfo::IncomingResponse(_) => "Incoming Response",
            TrafficInfo::OutgoingResponse(_) => "Outgoing Response",
            TrafficInfo::OutgoingRequest(_) => "Outgoing Request",
            TrafficInfo::IncomingRequest(_) => "Incoming Request",
        };
        write!(f, "{}", info_string)
    }
}

impl<'a> From<&TrafficInfo> for Text<'a> {
    fn from(traffic_info: &TrafficInfo) -> Text<'a> {
        match traffic_info {
            TrafficInfo::OutgoingRequest(i) | TrafficInfo::IncomingRequest(i) => Text::from(i),
            TrafficInfo::OutgoingResponse(i) | TrafficInfo::IncomingResponse(i) => Text::from(i),
        }
    }
}
