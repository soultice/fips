use crate::utility::log::LoggableType;

use super::request_info::RequestInfoNT;
use super::response_info::ResponseInfoNT;
use std::fmt;
use gradient_tui_fork::text::Text;

#[derive(Debug, Clone)]
pub struct LoggableNT(pub LoggableType);

impl fmt::Display for LoggableNT {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let info_string = match &self.0 {
            LoggableType::IncomingResponseAtFips(_) => "Incoming Response at FIPS",
            LoggableType::OutgoingResponseAtFips(_) => "Outgoing Response from FIPS",
            LoggableType::OutgoingRequestToServer(_) => "Outgoing Request to Server",
            LoggableType::IncomingRequestAtFips(_) => "Incoming Request at FIPS",
            LoggableType::IncomingResponseFromServer(_) => "Incoming Response from Server",
            LoggableType::Plain => "",
        };
        write!(f, "{info_string}")
    }
}

impl<'a> From<&LoggableNT> for Text<'a> {
    fn from(traffic_info: &LoggableNT) -> Text<'a> {
        match &traffic_info.0 {
            LoggableType::OutgoingRequestToServer(i) | LoggableType::IncomingRequestAtFips(i) => {
                Text::from(&RequestInfoNT(i.clone()))
            }
            LoggableType::IncomingResponseFromServer(i) |
            LoggableType::IncomingResponseAtFips(i) |
            LoggableType::OutgoingResponseAtFips(i) => Text::from(&ResponseInfoNT(i.clone())),
            LoggableType::Plain => Text::from(""),
        }
    }
}
