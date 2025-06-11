use hyper::body::Incoming;
use gradient_tui_fork::text::Text;
use crate::utility::log::RequestInfo;

type FipsBody = Incoming;

pub struct RequestInfoNT(pub RequestInfo);

impl From<&hyper::Request<FipsBody>> for RequestInfoNT {
    fn from(request: &hyper::Request<FipsBody>) -> RequestInfoNT {
        RequestInfoNT(RequestInfo {
            id: uuid::Uuid::new_v4().to_string(),
            request_type: String::from("HTTP"),
            method: request.method().clone(),
            uri: request.uri().clone(),
            version: request.version(),
            headers: request.headers().clone(),
        })
    }
}

impl<'a> From<&RequestInfoNT> for Text<'a> {
    fn from(request_info: &RequestInfoNT) -> Text<'a> {
        let mut text = String::new();
        text.push_str(request_info.0.method.as_str());
        text.push(' ');
        text.push_str(&request_info.0.uri.to_string());
        text.push(' ');
        text.push_str(&format!("{:?}", request_info.0.version));
        for (k, v) in request_info.0.headers.iter() {
            text.push('\n');
            text.push_str(k.as_str());
            text.push_str(" : ");
            text.push_str(v.to_str().unwrap_or("invalid header value"));
        }
        Text::from(text)
    }
}
