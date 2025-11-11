// Type definitions for logging, implementation is left to the respective crates
use std::collections::HashMap;
use hyper::{Response, Request};
use std::sync::atomic::{AtomicU64, Ordering};
use once_cell::sync::Lazy;

// Global atomic counter for correlation IDs (simple, fast). Could be replaced by UUID if needed.
static CORRELATION_COUNTER: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(1));

pub fn next_correlation_id() -> u64 {
    CORRELATION_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone)]
pub struct RequestInfo {
    pub request_type: String,
    pub method: String,
    pub uri: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub correlation_id: u64,
}

impl RequestInfo {
    pub fn from_with_id<B>(request: &Request<B>, correlation_id: u64) -> RequestInfo {
        let method = String::from(request.method().as_str());
        let uri = request.uri().to_string();
        let version = format!("{:?}", request.version());
        let mut headers = HashMap::new();
        for (k, v) in request.headers() {
            headers.insert(
                String::from(k.as_str()),
                String::from(v.to_str().unwrap()),
            );
        }
        RequestInfo{
            request_type: String::from("placeholder"),
            method,
            uri,
            version,
            headers,
            correlation_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResponseInfo {
    pub response_type: String,
    pub status: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub correlation_id: u64,
}

impl ResponseInfo {
    pub fn from_with_id<B>(response: &Response<B>, correlation_id: u64) -> ResponseInfo {
        let status = String::from(response.status().as_str());
        let version = format!("{:?}", response.version());
        let mut headers = HashMap::new();
        for (k, v) in response.headers() {
            headers.insert(
                String::from(k.as_str()),
                String::from(v.to_str().unwrap()),
            );
        }
        ResponseInfo{
            response_type: String::from("placeholder"),
            status,
            version,
            headers,
            correlation_id,
        }
    }
}

#[derive(Debug, Clone)]
pub enum LoggableType {
  IncomingRequestAtFfips(RequestInfo),
  OutGoingResponseFromFips(ResponseInfo),
  OutgoingRequestToServer(RequestInfo),
  IncomingResponseFromServer(ResponseInfo),
  Plain
}

pub struct Loggable {
  pub message_type: LoggableType,
  pub message: String,
}
