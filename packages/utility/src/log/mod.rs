// Type definitions for logging, implementation is left to the respective crates

pub enum LoggableType {
  IncomingRequestAtFfips,
  OutGoingResponseFromFips,
  OutgoingRequestToServer,
  IncomingResponseFromServer,
  Plain
}

pub struct Loggable {
  pub message_type: LoggableType,
  pub message: String,
}
