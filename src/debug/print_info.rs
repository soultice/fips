use hyper::{Body, Request, Response};
use std::collections::HashMap;
use tui::text::{Span, Spans};

pub enum TrafficInfo {
    INCOMING_REQUEST(RequestInfo),
    OUTGOING_REQUEST(RequestInfo),
    INCOMING_RESPONSE(ResponseInfo),
    OUTGOING_RESPONSE(ResponseInfo),
}

#[derive(Debug)]
pub struct ResponseInfo {
    status: String,
    version: String,
    headers: HashMap<String, String>,
}

impl<'a> From<&ResponseInfo> for Spans<'a> {
    fn from(response_info: &ResponseInfo) -> Spans<'a> {
        let mut info_vec = vec![
            Span::from(response_info.status.clone()),
            Span::from(response_info.version.clone()),
        ];
        for (k, v) in &response_info.headers {
            info_vec.push(Span::from(k.clone()));
            info_vec.push(Span::from(v.clone()));
        }
        Spans::from(info_vec)
    }
}

impl From<&Response<Body>> for ResponseInfo {
    fn from(response: &Response<Body>) -> ResponseInfo {
        let status = String::from(response.status().clone().as_str());
        let version = String::from(format!("{:?}", response.version().clone()));
        let mut headers = HashMap::new();
        for (k, v) in response.headers() {
            headers.insert(
                String::from(k.clone().as_str()),
                String::from(v.clone().to_str().unwrap()),
            );
        }
        ResponseInfo {
            status,
            version,
            headers,
        }
    }
}

#[derive(Debug)]
pub struct RequestInfo {
    method: String,
    uri: String,
    version: String,
    headers: HashMap<String, String>,
}

impl<'a> From<&RequestInfo> for Spans<'a> {
    fn from(request_info: &RequestInfo) -> Spans<'a> {
        let mut info_vec = vec![
            Span::from(request_info.method.clone()),
            Span::from(request_info.uri.clone()),
            Span::from(request_info.version.clone()),
        ];
        for (k, v) in &request_info.headers {
            info_vec.push(Span::from(k.clone()));
            info_vec.push(Span::from("\n"));
            info_vec.push(Span::from(v.clone()));
        }
        Spans::from(info_vec)
    }
}

impl From<&Request<Body>> for RequestInfo {
    fn from(request: &Request<Body>) -> RequestInfo {
        let method = String::from(request.method().clone().as_str());
        let uri = String::from(request.uri().clone().to_string());
        let version = String::from(format!("{:?}", request.version().clone()));
        let mut headers = HashMap::new();
        for (k, v) in request.headers() {
            headers.insert(
                String::from(k.clone().as_str()),
                String::from(v.clone().to_str().unwrap()),
            );
        }
        RequestInfo {
            method,
            uri,
            version,
            headers,
        }
    }
}

pub enum PrintInfo {
    PLAIN(String),
    MOXY(MoxyInfo),
}

pub struct MoxyInfo {
    pub method: String,
    pub path: String,
    pub mode: String,
    pub matching_rules: usize,
    pub response_code: String,
}

impl<'a> From<&MoxyInfo> for Spans<'a> {
    fn from(moxy_info: &MoxyInfo) -> Spans<'a> {
        Spans::from(vec![
            Span::from(moxy_info.method.to_owned()),
            Span::from(" "),
            Span::from("Mode: "),
            Span::from(moxy_info.mode.to_owned()),
            Span::from("=> "),
            Span::from(moxy_info.response_code.to_owned()),
            Span::from(" "),
            Span::from("Matched Rules: "),
            Span::from(moxy_info.matching_rules.to_owned().to_string()),
            Span::from(" "),
            Span::from(moxy_info.path.to_owned()),
        ])
    }
}
