use crate::client::AppClient;
use crate::configuration::{Mode, Rule};
use crate::debug::{MoxyInfo, PrintInfo};
use crate::{MainError, State};
use hyper::body::Bytes;
use hyper::header::HeaderValue;
use hyper::http::header::HeaderName;
use hyper::http::request::Parts;
use hyper::{Body, Method, Response, StatusCode, Uri};
use json_dotpath::DotPaths;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

struct Mox;

impl Mox {
    pub async fn forward_request(
        uri: &Uri,
        method: &Method,
        headers: Option<Vec<String>>,
        body: Bytes,
        parts: &Parts,
        state: &Arc<State>,
    ) -> Result<(hyper::http::response::Parts, Value), MainError> {
        let mut client = AppClient {
            uri,
            method,
            headers,
            body,
            parts,
        };

        Ok(client.response(state).await?)
    }

    pub fn set_status(
        response_status: &Option<u16>,
        returned_response: &mut Response<Body>,
    ) -> Result<(), MainError> {
        if let Some(response_status) = response_status {
            *returned_response.status_mut() = StatusCode::from_u16(*response_status)?
        }
        Ok(())
    }

    // Apply transformation from rules
    pub fn transform_response(
        rules: &Option<Vec<Rule>>,
        resp_json: &mut serde_json::Value,
    ) -> Result<(), MainError> {
        if let Some(rules) = rules {
            for rule in rules {
                resp_json.dot_set(&rule.path, rule.item.clone())?;
            }
        }
        Ok(())
    }

    pub fn keep_headers(
        backwards_headers: &Option<Vec<String>>,
        returned_response: &mut Response<Body>,
    ) -> Result<(), MainError> {
        if let Some(backward_headers) = backwards_headers {
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
        Ok(())
    }

    pub fn add_headers(
        headers: &Option<HashMap<String, String>>,
        returned_response: &mut Response<Body>,
    ) -> Result<(), MainError> {
        if let Some(headers) = headers {
            for header in headers {
                let header_name = HeaderName::from_str(header.0)?;
                let header_value = HeaderValue::from_str(header.1)?;
                returned_response
                    .headers_mut()
                    .insert(header_name, header_value);
            }
        }
        Ok(())
    }
}

pub async fn moxy<'r>(
    body: Body,
    parts: Parts,
    state: &Arc<State>,
) -> Result<Response<Body>, MainError> {
    let method = &parts.method;
    let uri = &parts.uri;
    let matches = state
        .configuration
        .lock()
        .unwrap()
        .active_matching_rules(&uri);

    let (mut returned_response, mode) = match matches.len() {
        0 => {
            let mut response = Response::new(Body::from("no matching rule found"));
            *response.status_mut() = StatusCode::NOT_FOUND;
            (response, Mode::PROXY)
        }
        _ => {
            let mut first_matched_rule = state.configuration.lock().unwrap().clone_rule(matches[0]);

            let mode: Mode = first_matched_rule.mode();

            let mut returned_response = match mode {
                Mode::PROXY | Mode::MOXY => {
                    let uri = &first_matched_rule.forward_url(&uri);

                    let body = hyper::body::to_bytes(body).await?;
                    let (client_parts, mut resp_json) = Mox::forward_request(
                        uri,
                        method,
                        first_matched_rule.forward_headers.clone(),
                        body,
                        &parts,
                        state,
                    )
                    .await?;

                    first_matched_rule.expand_rule_template(&state.plugins.lock().unwrap());

                    // if the response can not be transformed we do nothing
                    Mox::transform_response(&first_matched_rule.rules, &mut resp_json)
                        .unwrap_or_default();

                    let final_response_string = serde_json::to_string(&resp_json)?;
                    let returned_response = match final_response_string {
                        s if s.is_empty() => {
                            Response::from_parts(client_parts, Body::from(Body::default()))
                        }
                        s => Response::from_parts(client_parts, Body::from(s.clone())),
                    };
                    returned_response
                }
                Mode::MOCK => {
                    first_matched_rule.expand_rule_template(&state.plugins.lock().unwrap());
                    let body = Body::from(serde_json::to_string(
                        &first_matched_rule.rules.as_ref().unwrap()[0].item,
                    )?);
                    let returned_response = Response::new(body);
                    returned_response
                }
            };

            Mox::keep_headers(&first_matched_rule.backward_headers, &mut returned_response)?;

            Mox::add_headers(&first_matched_rule.headers, &mut returned_response)?;

            Mox::set_status(&first_matched_rule.response_status, &mut returned_response)?;
            // Add or change response status
            (returned_response, mode)
        }
    };

    state
        .add_message(PrintInfo::MOXY(MoxyInfo {
            method: method.to_string(),
            path: uri.to_string(),
            mode: mode.to_string(),
            matching_rules: matches.len(),
            response_code: returned_response.status().to_string(),
        }))
        .unwrap_or_default();

    returned_response
        .headers_mut()
        .insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    returned_response.headers_mut().insert(
        "Access-Control-Allow-Headers",
        HeaderValue::from_static("*"),
    );

    Ok(returned_response)
}
