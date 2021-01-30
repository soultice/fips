use hyper::{Body, Response};
use std::collections::HashMap;
use tui::text::Text;

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
