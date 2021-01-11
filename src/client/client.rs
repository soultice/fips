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

impl<'a> AppClient<'a> {
    pub async fn response(&mut self) -> Option<(Parts, serde_json::Value)> {
        let client = Client::new();
        let body_test = Body::from(self.body.clone());
        let mut client_req = hyper::Request::builder()
            .method(self.method.clone())
            .uri(self.uri)
            .body(body_test)
            .unwrap();

        if let Some(headers) = &self.headers {
            for header_name in headers {
                let header = HeaderName::from_str(&header_name).unwrap();
                let header_value = self.parts.headers.get(header_name).unwrap().clone();
                client_req.headers_mut().insert(header, header_value);
            }
        }

        let client_res = client.request(client_req).await.unwrap();
        let (client_parts, client_body) = client_res.into_parts();
        let body = hyper::body::aggregate(client_body).await.ok()?;
        let mut buffer = String::new();
        body.reader().read_to_string(&mut buffer).unwrap();
        let mut resp_json: serde_json::Value = serde_json::Value::from("");
        if !buffer.is_empty() {
            resp_json = serde_json::from_str(&buffer).unwrap();
        }
        Some((client_parts, resp_json))
    }
}
