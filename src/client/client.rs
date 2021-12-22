use crate::bytes::Buf;
use terminal_ui::debug::{RequestInfo, ResponseInfo, TrafficInfo};
use terminal_ui::state::State;
use hyper::body::Bytes;
use hyper::{header::HeaderName, http::response::Parts, Body, Client, Method, Uri};
use std::str::FromStr;
use std::sync::Arc;
use std::error::Error;

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
        state: &Arc<State>,
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
                let header = HeaderName::from_str(&header_name)?;
                let header_value = self.parts.headers.get(header_name);
                if let Some(hv) = header_value {
                    client_req.headers_mut().insert(header, hv.clone());
                }
            }
        }

        let outgoing_request_info = RequestInfo::from(&client_req);
        state
            .add_traffic_info(TrafficInfo::OutgoingRequest(outgoing_request_info))
            .unwrap_or_default();

        let client_res = client.request(client_req).await?;

        let incoming_response_info = ResponseInfo::from(&client_res);
        state
            .add_traffic_info(TrafficInfo::IncomingResponse(incoming_response_info))
            .unwrap_or_default();

        let (mut client_parts, client_body) = client_res.into_parts();

        //hyper creates them automatically and crashes in case of a mismatch, so removing them is the easiest way
        client_parts.headers.remove("content-length");

        let body = hyper::body::aggregate(client_body).await?.reader();
        let resp_json: serde_json::Value = serde_json::from_reader(body).unwrap_or_default();
        Ok((client_parts, resp_json))
    }
}
