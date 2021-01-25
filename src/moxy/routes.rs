use super::request::moxy;
use crate::debug::{RequestInfo, ResponseInfo, TrafficInfo};
use crate::State;
use hyper::header::HeaderValue;
use hyper::{Body, Method, Request, Response};
use std::sync::Arc;

pub async fn routes(req: Request<Body>, state: Arc<State>) -> Result<Response<Body>, hyper::Error> {
    let req_info = RequestInfo::from(&req);
    state
        .add_traffic_info(TrafficInfo::IncomingRequest(req_info))
        .unwrap_or_default();

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
            Ok(new_response)
        }

        _ => {
            let (parts, body) = req.into_parts();
            let resp: Response<Body> = moxy(body, parts, &state).await.unwrap();
            let response_info = ResponseInfo::from(&resp);
            state
                .add_traffic_info(TrafficInfo::OutgoingResponse(response_info))
                .unwrap_or_default();
            Ok(resp)
        }
    }
}
