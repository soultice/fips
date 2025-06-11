use hyper::body::Incoming;
use gradient_tui_fork::text::Text;
use crate::utility::log::ResponseInfo;

type FipsBody = Incoming;

pub struct ResponseInfoNT(pub ResponseInfo);

impl From<&hyper::Response<FipsBody>> for ResponseInfoNT {
    fn from(response: &hyper::Response<FipsBody>) -> ResponseInfoNT {
        ResponseInfoNT(ResponseInfo {
            status: response.status(),
            version: response.version(),
            headers: response.headers().clone(),
        })
    }
}

impl<'a> From<&ResponseInfoNT> for Text<'a> {
    fn from(response_info: &ResponseInfoNT) -> Text<'a> {
        let mut text = String::new();
        text.push_str(&response_info.0.status.to_string());
        text.push(' ');
        text.push_str(&format!("{:?}", response_info.0.version));
        for (k, v) in response_info.0.headers.iter() {
            text.push('\n');
            text.push_str(k.as_str());
            text.push_str(" : ");
            text.push_str(v.to_str().unwrap_or("invalid header value"));
        }
        Text::from(text)
    }
}

