use super::request::pimps;
use hyper::header::HeaderValue;
use hyper::{Body, Method, Request, Response, StatusCode};
use std::sync::Arc;
use terminal_ui::debug::{PrintInfo, RequestInfo, ResponseInfo, TrafficInfo};
use terminal_ui::state::State;

pub async fn routes(req: Request<Body>, state: Arc<State>) -> Result<Response<Body>, hyper::Error> {
    let req_info = RequestInfo::from(&req);

    state
        .add_traffic_info(TrafficInfo::IncomingRequest(req_info))
        .unwrap_or_default();

    let uri = req.uri().clone();
    let method = req.method().clone();
    let (parts, body) = req.into_parts();
    let body_bytes = hyper::body::to_bytes(body).await?;
    let body_text = String::from_utf8(body_bytes.to_vec()).unwrap();

    if method == "OPTIONS" {
        let mut preflight = Response::new(Body::default());
        preflight
            .headers_mut()
            .insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
        preflight.headers_mut().insert(
            "Access-Control-Allow-Headers",
            HeaderValue::from_static("*"),
        );
        preflight.headers_mut().insert(
            "Access-Control-Allow-Methods",
            HeaderValue::from_static("*"),
        );
        return Ok(preflight);
    }

    let matching_rules =
        state
            .configuration
            .lock()
            .unwrap()
            .active_matching_rules(uri.path(), &method, &body_text);

    match matching_rules.len() {
        0 => {
            let mut no_matching_rule = Response::new(Body::from("no matching rule found"));
            *no_matching_rule.status_mut() = StatusCode::NOT_FOUND;
            no_matching_rule.headers_mut().insert(
                "Access-Control-Allow-Headers",
                HeaderValue::from_static("*"),
            );
            no_matching_rule.headers_mut().insert(
                "Access-Control-Allow-Methods",
                HeaderValue::from_static("*"),
            );
            state
                .add_message(PrintInfo::PLAIN(format!(
                    "No matching rule found for URI: {}",
                    &uri
                )))
                .unwrap_or_default();
            Ok(no_matching_rule)
        }

        _ => match (&method, uri.path()) {
            (&Method::GET, "/favicon.ico") => Ok(Response::new(Body::default())),

            _ => {
                let mut first_matched_rule = state
                    .configuration
                    .lock()
                    .unwrap()
                    .clone_rule(matching_rules[0]);

                let resp: Response<Body> =
                    pimps(body_bytes, parts, &state, &mut first_matched_rule)
                        .await
                        .unwrap();

                let response_info = ResponseInfo::from(&resp);

                state
                    .add_traffic_info(TrafficInfo::OutgoingResponse(response_info))
                    .unwrap_or_default();

                if let Some(sleep) = first_matched_rule.sleep {
                    tokio::time::sleep(tokio::time::Duration::from_millis(sleep)).await;
                }
                Ok(resp)
            }
        },
    }
}
