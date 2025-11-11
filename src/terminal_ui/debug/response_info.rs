use gradient_tui_fork::text::Text;
use std::collections::HashMap;
use hyper::Response;

use crate::utility::log::ResponseInfo;

pub struct ResponseInfoNT(pub ResponseInfo);

impl<B> From<&Response<B>> for ResponseInfoNT {
    fn from(response: &Response<B>) -> ResponseInfoNT {
        let status = String::from(response.status().as_str());
        let version = format!("{:?}", response.version());
        let mut headers = HashMap::new();
        for (k, v) in response.headers() {
            headers.insert(
                String::from(k.as_str()),
                String::from(v.to_str().unwrap()),
            );
        }
        ResponseInfoNT(ResponseInfo {
            response_type: String::from("placeholder"),
            status,
            version,
            headers,
            correlation_id: 0, // UI-created without correlation context
        })
    }
}

impl<'a> From<&ResponseInfoNT> for Text<'a> {
    fn from(request_info: &ResponseInfoNT) -> Text<'a> {
        let mut text = format!("[cid={}] {}", request_info.0.correlation_id, request_info.0.status);
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

