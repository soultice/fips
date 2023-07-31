use hyper::{Body, Request};
use std::collections::HashMap;
use gradient_tui_fork::text::Text;

use crate::utility::log::RequestInfo;

pub struct RequestInfoNT(pub RequestInfo);

impl From<&Request<Body>> for RequestInfoNT {
    fn from(request: &Request<Body>) -> RequestInfoNT {
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
        RequestInfoNT(RequestInfo {
            request_type: String::from("placeholder"),
            method,
            uri,
            version,
            headers,
        })
    }
}

impl<'a> From<&RequestInfoNT> for Text<'a> {
    fn from(request_info: &RequestInfoNT) -> Text<'a> {
        let mut text = String::from(&request_info.0.method);
        text.push(' ');
        text.push_str(&request_info.0.uri);
        text.push(' ');
        text.push_str(&request_info.0.version);
        for (k, v) in &request_info.0.headers {
            text += "\n";
            text += k;
            text += " : ";
            text += v;
        }
        Text::from(text)
    }
}
