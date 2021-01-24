use crate::bytes::Buf;
use crate::debug::{RequestInfo, ResponseInfo, TrafficInfo};
use crate::State;
use hyper::{header::HeaderName, http::response::Parts, Body, Client, Method, Uri};
use std::io::Read;
use std::str::FromStr;
use std::sync::Arc;

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
    pub async fn response(
        &mut self,
        state: &Arc<State>,
    ) -> Result<(Parts, serde_json::Value), ClientError> {
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

        let mut outgoing_request_info = RequestInfo::from(&client_req);
        outgoing_request_info.request_type = String::from("Request to Server");
        state
            .traffic_info
            .lock()
            .unwrap()
            .push(TrafficInfo::OUTGOING_REQUEST(outgoing_request_info));

        let client_res = client.request(client_req).await?;

        let mut incoming_response_info = ResponseInfo::from(&client_res);
        incoming_response_info.response_type = String::from("Response From Server");

        state
            .traffic_info
            .lock()
            .unwrap()
            .push(TrafficInfo::INCOMING_RESPONSE(incoming_response_info));

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
