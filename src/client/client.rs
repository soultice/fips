use hyper::{Client, Body, Method, Request, Response, Server, StatusCode, http::{HeaderValue, response::Parts}, header::{HeaderName}, service::{make_service_fn, service_fn}};
use std::io::Read;
use std::str::FromStr;
use crate::bytes::Buf;

#[derive(Debug)]
pub struct AppClient<'a>
{
    pub uri: &'a str,
    pub method: &'a Method,
    pub headers: Option<Vec<String>>,
    pub body: String,
    pub parts: &'a hyper::http::request::Parts
}

impl <'a> AppClient<'a> {
    pub async fn response(&mut self) -> Option<(Parts, serde_json::Value)> {
        let client = Client::new();
        let body_test = Body::from(self.body.clone());
        let mut client_req = hyper::Request::builder().method(self.method.clone()).uri(self.uri).body(body_test).unwrap();

        if let Some(headers) = &self.headers {
            for header_name in headers {
                //println!("header-name {}", header_name);
                let header = HeaderName::from_str(&header_name).unwrap();
                let header_value = self.parts.headers.get(header_name).unwrap().clone();
                client_req.headers_mut().insert(header, header_value);
            }
        }

        let client_res = client.request(client_req).await.unwrap();
        let (mut client_parts, client_body) = client_res.into_parts();
        //println!("{:?}", client_parts.headers);
        let body = hyper::body::aggregate(client_body).await.unwrap();
        let mut buffer = String::new();
        body.reader().read_to_string(&mut buffer);
        let mut resp_json: serde_json::Value = serde_json::Value::from("");
        if !buffer.is_empty() {
            resp_json = serde_json::from_str(&buffer).unwrap();
        }
        Some((client_parts, resp_json))
    }

}

