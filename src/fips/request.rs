use crate::client::AppClient;
use crate::PaintLogsCallbacks;
use configuration::{Mode, Rule, RuleCollection};
use hyper::body::Bytes;
use hyper::header::HeaderValue;
use hyper::http::header::HeaderName;
use hyper::http::request::Parts;
use hyper::{Body, Method, Response, StatusCode, Uri};
use json_dotpath::DotPaths;
use plugin_registry::ExternalFunctions;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use utility::log::{Loggable, LoggableType};

struct Fips;

impl Fips {
    pub async fn forward_request<'a>(
        uri: &Uri,
        method: &Method,
        headers: Option<Vec<String>>,
        body: Bytes,
        parts: &Parts,
        logging: &PaintLogsCallbacks,
    ) -> Result<(hyper::http::response::Parts, Value), Box<dyn Error>> {
        let mut client = AppClient {
            uri,
            method,
            headers,
            body,
            parts,
        };

        client.response(logging).await
    }

    pub fn set_status(
        response_status: &Option<u16>,
        returned_response: &mut Response<Body>,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(response_status) = response_status {
            *returned_response.status_mut() = StatusCode::from_u16(*response_status)?
        }
        Ok(())
    }

    // Apply transformation from rules
    pub fn transform_response(
        rules: &Option<Vec<Rule>>,
        resp_json: &mut serde_json::Value,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(rules) = rules {
            for rule in rules {
                match &rule.path {
                    Some(path) => {
                        resp_json.dot_set(path, rule.item.clone())?;
                    }
                    None => {
                        resp_json.dot_set("", rule.item.clone())?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn keep_headers(
        backwards_headers: &Option<Vec<String>>,
        returned_response: &mut Response<Body>,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(backward_headers) = backwards_headers {
            let mut header_buffer: Vec<(HeaderName, HeaderValue)> = Vec::new();
            for header_name in backward_headers {
                let header = HeaderName::from_str(header_name)?;
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
    ) -> Result<(), Box<dyn Error>> {
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

pub async fn handle_mode(
    body: Bytes,
    parts: Parts,
    plugins: &Arc<Mutex<ExternalFunctions>>,
    first_matched_rule: &mut RuleCollection,
    logging: &PaintLogsCallbacks,
) -> Result<Response<Body>, Box<dyn Error>> {
    let method = &parts.method;
    let uri = &parts.uri;
    let headers = &parts.headers;

    let mode: Mode = first_matched_rule.mode();

    let mut returned_response = match mode {
        Mode::PROXY | Mode::FIPS => {
            let uri = &first_matched_rule.forward_url(uri);

            let (client_parts, mut resp_json) = Fips::forward_request(
                uri,
                method,
                first_matched_rule.forward_headers.clone(),
                body,
                &parts,
                logging,
            )
            .await?;

            first_matched_rule.expand_rule_template(&plugins.lock().unwrap());

            // if the response can not be transformed we do nothing
            Fips::transform_response(&first_matched_rule.rules, &mut resp_json).unwrap_or_default();

            let final_response_string = serde_json::to_string(&resp_json)?;
            match final_response_string {
                s if s.is_empty() => Response::from_parts(client_parts, Body::default()),
                s => Response::from_parts(client_parts, Body::from(s)),
            }
        }
        Mode::MOCK => {
            first_matched_rule.expand_rule_template(&plugins.lock().unwrap());
            let body = match &first_matched_rule.rules {
                Some(rules) => match rules.len() {
                    0 => Response::new(Body::default()),
                    _ => {
                        let body = Body::from(serde_json::to_string(&rules[0].item)?);
                        Response::new(body)
                    }
                },
                None => Response::new(Body::default()),
            };
            body
        }
        Mode::STATIC => {
            let result = hyper_staticfile::resolve_path(
                &first_matched_rule.serve_static.clone().unwrap(),
                &parts.uri.to_string(),
            )
            .await?;
            hyper_staticfile::ResponseBuilder::new()
                .request_parts(method, uri, headers)
                .build(result)?
        }
    };

    Fips::keep_headers(&first_matched_rule.backward_headers, &mut returned_response)?;

    Fips::add_headers(&first_matched_rule.headers, &mut returned_response)?;

    Fips::set_status(&first_matched_rule.response_status, &mut returned_response)?;
    // Add or change response status

    let _name = first_matched_rule.name.clone().unwrap_or(String::from(""));

    let info = Loggable {
        message_type: LoggableType::Plain,
        message: format!("Request {method} {uri} {mode} {_name}"),
    };
    (logging.0)(&info);

    returned_response
        .headers_mut()
        .insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    returned_response.headers_mut().insert(
        "Access-Control-Allow-Headers",
        HeaderValue::from_static("*"),
    );

    Ok(returned_response)
}
