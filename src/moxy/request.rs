use crate::client::AppClient;
use crate::configuration::Mode;
use crate::debug::{MoxyInfo, PrintInfo};
use crate::{MainError, State};
use bytes::Buf;
use hyper::header::HeaderValue;
use hyper::http::header::HeaderName;
use hyper::{Body, Response, StatusCode};
use json_dotpath::DotPaths;
use std::io::Read;
use std::str::FromStr;
use std::sync::Arc;

pub async fn moxy<'r>(
    body: Body,
    parts: hyper::http::request::Parts,
    state: &Arc<State>,
) -> Result<Response<Body>, MainError> {
    let method = &parts.method;
    let uri = &parts.uri;
    let matches = state.configuration.lock().unwrap().matching_rules(&uri);

    let (mut returned_response, mode) = match matches.len() {
        0 => {
            let mut response = Response::new(Body::from("no matching rule found"));
            *response.status_mut() = StatusCode::NOT_FOUND;
            (response, Mode::PROXY)
        }
        _ => {
            let mut first_matched_rule = state
                .configuration
                .lock()
                .unwrap()
                .clone_collection(matches[0]);

            let mode: Mode = first_matched_rule.mode();

            let mut returned_response = match mode {
                Mode::PROXY | Mode::MOXY => {
                    let uri = &first_matched_rule.forward_url(&uri);

                    let body_str = hyper::body::aggregate(body).await?;
                    let mut buffer = String::new();
                    body_str.reader().read_to_string(&mut buffer)?;

                    let mut client = AppClient {
                        uri,
                        method,
                        headers: first_matched_rule.forward_headers.clone(),
                        body: buffer,
                        parts: &parts,
                    };

                    let (client_parts, mut resp_json) = client.response(state).await?;

                    first_matched_rule.expand_rule_template(&state.plugins.lock().unwrap());

                    // Apply transformation from rules
                    if let Some(rules) = &first_matched_rule.rules {
                        for rule in rules {
                            resp_json.dot_set(&rule.path, rule.item.clone())?;
                        }
                    }

                    let final_response_string = serde_json::to_string(&resp_json)?;
                    let returned_response = match final_response_string {
                        s if s.is_empty() => {
                            Response::from_parts(client_parts, Body::from(Body::default()))
                        }
                        s => Response::from_parts(client_parts, Body::from(s.clone())),
                    };
                    returned_response
                }
                _ => {
                    first_matched_rule.expand_rule_template(&state.plugins.lock().unwrap());
                    let body = Body::from(serde_json::to_string(
                        &first_matched_rule.rules.as_ref().unwrap()[0].item,
                    )?);
                    let returned_response = Response::new(body);
                    returned_response
                }
            };

            // Keep these headers from the original response
            if let Some(backward_headers) = &first_matched_rule.backward_headers {
                let mut header_buffer: Vec<(HeaderName, HeaderValue)> = Vec::new();
                for header_name in backward_headers {
                    let header = HeaderName::from_str(&header_name)?;
                    let header_value = returned_response
                        .headers()
                        .get(header_name)
                        .unwrap()
                        .clone();
                    header_buffer.push((header, header_value));
                }
                returned_response.headers_mut().clear();
                for header_tup in header_buffer {
                    returned_response
                        .headers_mut()
                        .insert(header_tup.0, header_tup.1);
                }
            }

            // Add headers to response
            if let Some(headers) = &first_matched_rule.headers {
                for header in headers {
                    let header_name = HeaderName::from_str(header.0)?;
                    let header_value = HeaderValue::from_str(header.1)?;
                    returned_response
                        .headers_mut()
                        .insert(header_name, header_value);
                }
            }

            // Add or change response status
            if let Some(response_status) = &first_matched_rule.response_status {
                *returned_response.status_mut() = StatusCode::from_u16(*response_status)?
            }
            (returned_response, mode)
        }
    };

    state.messages.lock().unwrap().insert(
        0,
        PrintInfo::MOXY(MoxyInfo {
            method: method.to_string(),
            path: uri.to_string(),
            mode: mode.to_string(),
            matching_rules: matches.len(),
            response_code: returned_response.status().to_string(),
        }),
    );

    returned_response
        .headers_mut()
        .insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    returned_response.headers_mut().insert(
        "Access-Control-Allow-Headers",
        HeaderValue::from_static("*"),
    );

    Ok(returned_response)
}
