use crate::bytes::Buf;
use hyper::{header::HeaderName, http::response::Parts, Body, Client, Method, Uri};
use std::io::Read;
use std::str::FromStr;

#[derive(Debug)]
pub struct AppClient<'a> {
    pub uri: &'a Uri,
    pub method: &'a Method,
    pub headers: Option<Vec<String>>,
    pub body: String,
    pub parts: &'a hyper::http::request::Parts,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClientError {
    Other { msg: String },
}

impl<S: ToString> From<S> for ClientError {
    fn from(other: S) -> ClientError {
        ClientError::Other {
            msg: other.to_string(),
        }
    }
}

impl<'a> AppClient<'a> {
    pub async fn response(&mut self) -> Result<(Parts, serde_json::Value), ClientError> {
        let client = Client::new();
        let body = Body::from(self.body.clone());
        let mut client_req = hyper::Request::builder()
            .method(self.method.clone())
            .uri(self.uri)
            .body(body)
            .unwrap();

        if let Some(headers) = &self.headers {
            for header_name in headers {
                let header = HeaderName::from_str(&header_name)?;
                let header_value = self.parts.headers.get(header_name);
                if let Some(hv) = header_value {
                    client_req.headers_mut().insert(header, hv.clone());
                }
            }
        }

        let client_res = client.request(client_req).await?;
        let (mut client_parts, client_body) = client_res.into_parts();

        //hyper creates them automatically and crashes in case of a mismatch, so removing them is the easiest way
        client_parts.headers.remove("content-length");

        let body = hyper::body::aggregate(client_body).await?;
        let mut buffer = String::new();
        body.reader().read_to_string(&mut buffer)?;
        let mut resp_json: serde_json::Value = serde_json::Value::from("");
        if !buffer.is_empty() {
            resp_json = serde_json::from_str(&buffer)?;
        }
        Ok((client_parts, resp_json))
    }
}
