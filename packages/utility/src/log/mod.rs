// Type definitions for logging, implementation is left to the respective crates
use std::collections::HashMap;
use hyper::{Body, Response, Request};

#[derive(Debug, Clone)]
pub struct RequestInfo {
    pub request_type: String,
    pub method: String,
    pub uri: String,
    pub version: String,
    pub headers: HashMap<String, String>,
}

impl From<&Request<Body>> for RequestInfo {
    fn from(request: &Request<Body>) -> RequestInfo {
        let method = String::from(request.method().clone().as_str());
        let uri = request.uri().clone().to_string();
        let version = format!("{:?}", request.version().clone());
        let mut headers = HashMap::new();
        for (k, v) in request.headers() {
            headers.insert(
                String::from(k.clone().as_str()),
                String::from(v.clone().to_str().unwrap()),
            );
        }
        RequestInfo{
            request_type: String::from("placeholder"),
            method,
            uri,
            version,
            headers,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResponseInfo {
    pub response_type: String,
    pub status: String,
    pub version: String,
    pub headers: HashMap<String, String>,
}

impl From<&Response<Body>> for ResponseInfo {
    fn from(response: &Response<Body>) -> ResponseInfo {
        let status = String::from(response.status().clone().as_str());
        let version = format!("{:?}", response.version().clone());
        let mut headers = HashMap::new();
        for (k, v) in response.headers() {
            headers.insert(
                String::from(k.clone().as_str()),
                String::from(v.clone().to_str().unwrap()),
            );
        }
        ResponseInfo{
            response_type: String::from("placeholder"),
            status,
            version,
            headers,
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
