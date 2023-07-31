use crate::PaintLogsCallbacks;
use crate::utility::log::Loggable;
use crate::utility::log::LoggableType;
use crate::utility::log::RequestInfo;
use crate::utility::log::ResponseInfo;
use hyper::body::Buf;
use hyper::body::Bytes;
use hyper::{header::HeaderName, http::response::Parts, Body, Client, Method, Uri};
use std::error::Error;
use std::str::FromStr;

#[derive(Debug)]
pub struct AppClient<'a> {
    pub uri: &'a Uri,
    pub method: &'a Method,
    pub headers: Option<Vec<String>>,
    pub body: Bytes,
    pub parts: &'a hyper::http::request::Parts,
}

impl<'a> AppClient<'a> {
    pub async fn response(
        &mut self,
        _logging: &PaintLogsCallbacks,
    ) -> Result<(Parts, serde_json::Value), Box<dyn Error>> {
        let client = Client::new();
        let body = Body::from(self.body.clone());

        let mut client_req = hyper::Request::builder()
            .method(self.method.clone())
            .uri(self.uri)
            .body(body)
            .unwrap();

        if let Some(headers) = &self.headers {
            for header_name in headers {
                let header = HeaderName::from_str(header_name)?;
                let header_value = self.parts.headers.get(header_name);
                if let Some(hv) = header_value {
                    client_req.headers_mut().insert(header, hv.clone());
                }
            }
        }

        (_logging.0)(&Loggable {
            message_type: LoggableType::OutgoingRequestToServer(RequestInfo::from(&client_req)),
            message: "".to_string(),
        });

        let client_res = client.request(client_req).await?;

        (_logging.0)(&Loggable {
            message_type: LoggableType::IncomingResponseFromServer(ResponseInfo::from(&client_res)),
            message: "".to_string(),
        });

        let (mut client_parts, client_body) = client_res.into_parts();

        //hyper creates them automatically and crashes in case of a mismatch, so removing them is the easiest way
        client_parts.headers.remove("content-length");

        let body = hyper::body::aggregate(client_body).await?.reader();
        let resp_json: serde_json::Value = serde_json::from_reader(body).unwrap_or_default();
        Ok((client_parts, resp_json))
    }
}
