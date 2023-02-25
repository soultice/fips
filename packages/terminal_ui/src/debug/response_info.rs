use fips_utility::log::ResponseInfo;
use gradient_tui_fork::text::Text;
use std::collections::HashMap;
use hyper::{Body, Response};

pub struct ResponseInfoNT(pub ResponseInfo);

impl From<&Response<Body>> for ResponseInfoNT {
    fn from(response: &Response<Body>) -> ResponseInfoNT {
        let status = String::from(response.status().clone().as_str());
        let version = format!("{:?}", response.version().clone());
        let mut headers = HashMap::new();
        for (k, v) in response.headers() {
            headers.insert(
                String::from(k.clone().as_str()),
                String::from(v.clone().to_str().unwrap()),
            );
        }
        ResponseInfoNT(ResponseInfo {
            response_type: String::from("placeholder"),
            status,
            version,
            headers,
        })
    }
}

impl<'a> From<&ResponseInfoNT> for Text<'a> {
    fn from(request_info: &ResponseInfoNT) -> Text<'a> {
        let mut text = String::from(&request_info.0.status);
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

