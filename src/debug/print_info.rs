use hyper::{Body, Request, Response};
use std::collections::HashMap;
use std::fmt;
use tui::text::{Span, Spans, Text};

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

#[derive(Debug, Clone)]
pub struct ResponseInfo {
    pub response_type: String,
    status: String,
    version: String,
    headers: HashMap<String, String>,
}

impl<'a> From<&ResponseInfo> for Text<'a> {
    fn from(request_info: &ResponseInfo) -> Text<'a> {
        let mut text = String::from(&request_info.status);
        text.push_str(" ");
        text.push_str(&request_info.version);
        for (k, v) in &request_info.headers {
            text += "\n";
            text += &k;
            text += " : ";
            text += &v;
        }
        Text::from(text)
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
            response_type: String::from("placeholder"),
            status,
            version,
            headers,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestInfo {
    pub request_type: String,
    method: String,
    uri: String,
    version: String,
    headers: HashMap<String, String>,
}

impl<'a> From<&RequestInfo> for Text<'a> {
    fn from(request_info: &RequestInfo) -> Text<'a> {
        let mut text = String::from(&request_info.method);
        text.push_str(" ");
        text.push_str(&request_info.uri);
        text.push_str(" ");
        text.push_str(&request_info.version);
        for (k, v) in &request_info.headers {
            text += "\n";
            text += &k;
            text += " : ";
            text += &v;
        }
        Text::from(text)
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
            request_type: String::from("placeholder"),
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
