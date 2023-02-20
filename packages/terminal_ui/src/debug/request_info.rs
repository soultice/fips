use hyper::{Body, Request};
use std::collections::HashMap;
use tui::text::Text;

#[derive(Debug, Clone)]
pub struct RequestInfo {
    pub request_type: String,
    pub method: String,
    pub uri: String,
    pub version: String,
    pub headers: HashMap<String, String>,
}

impl<'a> From<&RequestInfo> for Text<'a> {
    fn from(request_info: &RequestInfo) -> Text<'a> {
        let mut text = String::from(&request_info.method);
        text.push(' ');
        text.push_str(&request_info.uri);
        text.push(' ');
        text.push_str(&request_info.version);
        for (k, v) in &request_info.headers {
            text += "\n";
            text += k;
            text += " : ";
            text += v;
        }
        Text::from(text)
    }
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
        RequestInfo {
            request_type: String::from("placeholder"),
            method,
            uri,
            version,
            headers,
        }
    }
}
