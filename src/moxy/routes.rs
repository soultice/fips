use super::request::moxy;
use crate::debug::{RequestInfo, ResponseInfo, TrafficInfo};
use crate::State;
use hyper::header::HeaderValue;
use hyper::{Body, Method, Request, Response};
use std::sync::Arc;

pub async fn routes(req: Request<Body>, state: Arc<State>) -> Result<Response<Body>, hyper::Error> {
    let mut req_info = RequestInfo::from(&req);
    req_info.request_type = String::from("Request To Moxy");

    state
        .traffic_info
        .lock()
        .unwrap()
        .push(TrafficInfo::INCOMING_REQUEST(req_info));

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/favicon.ico") => Ok(Response::new(Body::from(""))),

        (&Method::OPTIONS, _) => {
            let mut new_response = Response::new(Body::from(""));
            new_response
                .headers_mut()
                .insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
            new_response.headers_mut().insert(
                "Access-Control-Allow-Headers",
                HeaderValue::from_static("*"),
            );
            new_response.headers_mut().insert(
                "Access-Control-Allow-Methods",
                HeaderValue::from_static("*"),
            );
            let response_info = ResponseInfo::from(&new_response);
            Ok(new_response)
        }

        _ => {
            let (parts, body) = req.into_parts();
            let resp: Response<Body> = moxy(body, parts, &state).await.unwrap();
            let mut response_info = ResponseInfo::from(&resp);
            response_info.response_type = String::from("Returned By Moxy");
            state
                .traffic_info
                .lock()
                .unwrap()
                .push(TrafficInfo::OUTGOING_RESPONSE(response_info));
            Ok(resp)
        }
    }
}
