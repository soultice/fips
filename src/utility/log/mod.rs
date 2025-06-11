// Type definitions for logging, implementation is left to the respective crates
use std::collections::HashMap;
use bytes::Bytes;
use hyper::{Request, Response, Version};
use hyper::body::Incoming;
use hyper::http::{HeaderMap, Method, StatusCode, Uri};
use http_body_util::combinators::BoxBody;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum LoggableType {
    IncomingRequestAtFips(RequestInfo),
    OutgoingResponseAtFips(ResponseInfo),
    IncomingResponseAtFips(ResponseInfo),
    OutgoingRequestToServer(RequestInfo),
    IncomingResponseFromServer(ResponseInfo),
    Plain,
}

#[derive(Debug, Clone)]
pub struct Loggable {
    pub message_type: LoggableType,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct RequestInfo {
    pub id: String,
    pub request_type: String,
    pub method: Method,
    pub uri: Uri,
    pub version: Version,
    pub headers: HeaderMap,
}

impl From<&Request<Incoming>> for RequestInfo {
    fn from(req: &Request<Incoming>) -> Self {
        RequestInfo {
            id: Uuid::new_v4().to_string(),
            request_type: "HTTP".to_string(),
            method: req.method().clone(),
            uri: req.uri().clone(),
            version: req.version(),
            headers: req.headers().clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResponseInfo {
    pub status: StatusCode,
    pub version: Version,
    pub headers: HeaderMap,
}

impl From<&Response<BoxBody<Bytes, hyper::Error>>> for ResponseInfo {
    fn from(resp: &Response<BoxBody<Bytes, hyper::Error>>) -> Self {
        ResponseInfo {
            status: resp.status(),
            version: resp.version(),
            headers: resp.headers().clone(),
        }
    }
}

// Type aliases for body types
pub type RequestBody = Incoming;
pub type ResponseBody = BoxBody<Bytes, hyper::Error>;
