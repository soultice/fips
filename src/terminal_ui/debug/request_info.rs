use hyper::Request;
use std::collections::HashMap;
use gradient_tui_fork::text::Text;

use crate::utility::log::RequestInfo;

pub struct RequestInfoNT(pub RequestInfo);

impl<B> From<&Request<B>> for RequestInfoNT {
    fn from(request: &Request<B>) -> RequestInfoNT {
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
