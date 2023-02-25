use crate::client::AppClient;
use crate::PaintLogsCallbacks;
use configuration::{
    rule::Rule,
    rule_collection::{ProxyFunctions, RuleCollection, RuleTransformingFunctions},
};
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

    let mut returned_response = match first_matched_rule {
        RuleCollection::Fips(r) => {
            let uri = &r.form_forward_path(uri)?;

            let (client_parts, mut resp_json) =
                Fips::forward_request(uri, method, r.get_forward_headers(), body, &parts, logging)
                    .await?;

            r.apply_plugins(&plugins.lock().unwrap());

            // if the response can not be transformed we do nothing
            Fips::transform_response(&r.rules, &mut resp_json).unwrap_or_default();

            let final_response_string = serde_json::to_string(&resp_json)?;
            match final_response_string {
                s if s.is_empty() => Response::from_parts(client_parts, Body::default()),
                s => Response::from_parts(client_parts, Body::from(s)),
            }
        }
        RuleCollection::Proxy(r) => {
            let uri = &r.form_forward_path(uri)?;

            let (client_parts, resp_json) =
                Fips::forward_request(uri, method, r.get_forward_headers(), body, &parts, logging)
                    .await?;

            let final_response_string = serde_json::to_string(&resp_json)?;
            match final_response_string {
                s if s.is_empty() => Response::from_parts(client_parts, Body::default()),
                s => Response::from_parts(client_parts, Body::from(s)),
            }
        }
        RuleCollection::Mock(r) => {
            r.apply_plugins(&plugins.lock().unwrap());
            match &r.rules.len() {
                0 => Response::new(Body::default()),
                _ => {
                    let body = Body::from(serde_json::to_string(&r.rules[0].item)?);
                    Response::new(body)
                }
            }
        }
        RuleCollection::Static(r) => {
            let result =
                hyper_staticfile::resolve_path(&r.static_base_dir.clone(), &parts.uri.to_string()).await?;
            hyper_staticfile::ResponseBuilder::new()
                .request_parts(method, uri, headers)
                .build(result)?
        }
    };

    match first_matched_rule {
        RuleCollection::Fips(r) => {
            Fips::keep_headers(&r.get_backward_headers(), &mut returned_response)?;
            Fips::add_headers(&r.headers, &mut returned_response)?;
            Fips::set_status(&r.response_status, &mut returned_response)?;
        }
        RuleCollection::Proxy(r) => {
            Fips::keep_headers(&r.get_backward_headers(), &mut returned_response)?;
            Fips::add_headers(&r.headers, &mut returned_response)?;
        }
        RuleCollection::Mock(r) => {
            Fips::add_headers(&r.headers, &mut returned_response)?;
            Fips::set_status(&r.response_status, &mut returned_response)?;
        }
        RuleCollection::Static(_) => {}
    }

    // Add or change response status

    let _name = first_matched_rule
        .get_name()
        .unwrap_or(String::from(""));

    let info = Loggable {
        message_type: LoggableType::Plain,
        message: format!("Request {method} {uri} {first_matched_rule} {_name}"),
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
