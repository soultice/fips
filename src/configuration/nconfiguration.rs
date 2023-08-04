use bytes::Buf;
use futures::StreamExt;
use http::{
    header::{self, HeaderName},
    method,
    request::Parts,
    response::Parts as ResponseParts,
    Extensions, HeaderMap, HeaderValue, Method, StatusCode, Uri,
};
use hyper::{Body, Request, Response};
use lazy_static::lazy_static;
use rand::Rng;
use regex::RegexSet;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("Could not parse URI: {0}")]
    UriParseError(#[from] hyper::http::uri::InvalidUri),
    #[error("Could not format String: {0}")]
    StringFromatError(#[from] std::fmt::Error),
    #[error("Invalid Body in rule: {0}")]
    InvalidBodyError(#[from] http::Error),
    #[error("Config has no uri")]
    NoUriError,
    #[error("Config has no method")]
    NoMethodError,
    #[error("Not forwarding as per rule definition")]
    NotForwarding,
}

use super::loader::{DeserializationError, YamlFileLoader};

lazy_static! {
    static ref HTTP_METHODS: Vec<String> = vec![
        String::from("GET"),
        String::from("OPTIONS"),
        String::from("POST"),
        String::from("PUT"),
        String::from("DELETE"),
        String::from("HEAD"),
        String::from("TRACE"),
        String::from("CONNECT"),
        String::from("PATCH"),
    ];
}

/*
Rule {
  when {
    matches
    bodyContains
    probability
  }
  then {
    type: Proxy / fips / mock / static
    forwardUri // only for proxy and fips
    modifyRequest // only for proxy and fips
    modifyResponse // only for proxy and fips
    ...
  }
  with {
    sleep
    plugins
  }
}
 */

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Match {
    uri: String,
    body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
enum RuleType {
    NonForwarding,
    Forwarding,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModifyRequest {
    #[serde(rename = "setHeaders")]
    add_headers: Option<HashMap<String, String>>,
    #[serde(rename = "keepHeaders")]
    keep_headers: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModifyResponse {
    #[serde(rename = "setHeaders")]
    add_headers: Option<HashMap<String, String>>,
    #[serde(rename = "keepHeaders")]
    keep_headers: Option<Vec<String>>,
    body: Option<Vec<BodyManipulation>>,
    status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BodyManipulation {
    at: String,
    with: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Plugin {
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct When {
    #[serde(rename = "matchesUris")]
    matches_uri: Vec<Match>,
    #[serde(rename = "matchesMethods")]
    matches_methods: Option<Vec<String>>,
    #[serde(rename = "bodyContains")]
    body_contains: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "functionAs")]
pub enum Then {
    Fips {
        #[serde(rename = "forwardUri")]
        forward_uri: String,
        #[serde(rename = "modifyResponse")]
        modify_response: Option<ModifyResponse>,
        status: Option<String>,
    },
    Proxy {
        #[serde(rename = "forwardUri")]
        forward_uri: String,
        status: Option<String>,
    },
    Static {
        #[serde(rename = "staticPath")]
        static_path: Option<String>,
    },
    Mock {
        body: Option<String>,
        status: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct With {
    pub sleep: Option<u64>,
    probability: Option<f32>,
    plugins: Option<Vec<Plugin>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum RuleSet {
    Rule(Rule),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Rule {
    pub name: String,
    pub when: When,
    pub then: Then,
    pub with: With,
}

impl Rule {
    pub fn should_apply(&self, intermediary: &Intermediary) -> bool {
        let mut rng = rand::thread_rng();

        let uri_regex = RegexSet::new(
            self.when
                .matches_uri
                .iter()
                .map(|m| m.uri.as_str())
                .collect::<Vec<&str>>(),
        )
        .unwrap();

        let uri = intermediary.clone().uri.unwrap();

        let some_uris_match = uri_regex.is_match(&uri);
        if !some_uris_match {
            return false;
        }

        let some_methods_match = self
            .when
            .matches_methods
            .as_ref()
            .map(|methods| {
                methods
                    .iter()
                    .any(|m| m == intermediary.clone().method.unwrap().as_str())
            })
            .unwrap_or(true);
        if !some_methods_match {
            return false;
        }

        let some_body_contains = self
            .when
            .body_contains
            .as_ref()
            .map(|body_contains| intermediary.body.as_str().unwrap().contains(body_contains))
            .unwrap_or(false);
        if !some_body_contains {
            return false;
        }

        let probability_matches = self
            .with
            .probability
            .as_ref()
            .map(|probability| {
                let random_number = rng.gen_range(0.0, 1.0);
                random_number < *probability
            })
            .unwrap_or(true);
        if !probability_matches {
            return false;
        }

        true
    }
}

#[derive(Deserialize, Debug, Clone, JsonSchema)]
pub struct NConfiguration {
    active_rule_indices: Vec<usize>,
    selected_rule: usize,
    pub rules: Vec<RuleSet>,
}

impl NConfiguration {
    pub fn load(paths: &Vec<PathBuf>) -> Result<NConfiguration, DeserializationError> {
        let extensions = vec![String::from("yaml"), String::from("yml")];
        let loader = YamlFileLoader { extensions };
        let rules = loader.load_from_directories::<RuleSet>(paths)?;
        Ok(NConfiguration {
            active_rule_indices: vec![0],
            selected_rule: 0,
            rules,
        })
    }

    pub fn select_next(&mut self) {
        self.selected_rule = (self.selected_rule + 1) % self.active_rule_indices.len();
    }

    pub fn select_previous(&mut self) {
        self.selected_rule = (self.selected_rule + self.active_rule_indices.len() - 1)
            % self.active_rule_indices.len();
    }

    pub fn remove_from_active_indices(&mut self) {
        self.active_rule_indices.remove(self.selected_rule);
    }

    pub fn add_to_active_indices(&mut self) {
        self.active_rule_indices.push(self.selected_rule);
    }
}

#[derive(Debug, Clone)]
pub struct Intermediary {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: serde_json::Value,
    pub method: Option<Method>,
    pub uri: Option<String>,
}

pub trait AsyncFrom<T> {
    type Output;
    async fn async_from(t: T) -> Self::Output;
}

impl AsyncFrom<hyper::Response<hyper::Body>> for Intermediary {
    type Output = Intermediary;

    async fn async_from(response: hyper::Response<hyper::Body>) -> Intermediary {
        let status = response.status();
        let mut headers = response.headers().clone();
        headers.remove("content-length");

        let body = response.into_body();
        let body = hyper::body::aggregate(body).await.unwrap().reader();
        let resp_json: serde_json::Value = serde_json::from_reader(body).unwrap_or_default();
        Intermediary {
            status,
            headers,
            body: resp_json,
            method: None,
            uri: None,
        }
    }
}

impl AsyncFrom<hyper::Request<hyper::Body>> for Intermediary {
    type Output = Intermediary;
    async fn async_from(request: hyper::Request<hyper::Body>) -> Intermediary {
        let method = request.method().clone();
        let uri = request.uri().to_string();
        let headers = request.headers().clone();
        let body = request.into_body();
        let body = hyper::body::aggregate(body).await.unwrap().reader();
        let req_json: serde_json::Value = serde_json::from_reader(body).unwrap_or_default();
        Intermediary {
            status: StatusCode::OK,
            headers,
            body: req_json,
            method: Some(method),
            uri: Some(uri),
        }
    }
}

impl TryFrom<Intermediary> for hyper::Request<hyper::Body> {
    type Error = ConfigurationError;
    fn try_from(intermediary: Intermediary) -> Result<Self, ConfigurationError> {
        let mut builder = Request::builder();
        if let Some(method) = intermediary.method {
            builder = builder.method(method);
        } else {
            return Err(ConfigurationError::NoMethodError);
        }
        if let Some(uri) = intermediary.uri {
            builder = builder.uri(uri);
        } else {
            return Err(ConfigurationError::NoUriError);
        }
        for (key, value) in intermediary.headers.iter() {
            builder = builder.header(key, value);
        }
        Ok(builder.body(Body::from(intermediary.body.to_string()))?)
    }
}

pub struct RuleAndIntermediaryHolder<'a> {
    pub rule: &'a Rule,
    pub intermediary: &'a Intermediary,
}

impl TryFrom<RuleAndIntermediaryHolder<'_>> for Request<Body> {
    type Error = ConfigurationError;

    fn try_from(holder: RuleAndIntermediaryHolder) -> Result<Self, ConfigurationError> {
        let header_iter = holder.intermediary.headers.iter().map(|(key, value)| {
            let key = HeaderName::from(key);
            let value = HeaderValue::from(value);
            (key, value)
        });
        let header_map: HeaderMap<HeaderValue> = HeaderMap::from_iter(header_iter);
        let uri = match &holder.rule.then {
            Then::Fips {
                forward_uri,
                modify_response,
                status,
            } => Ok(Uri::from_str(forward_uri)?),
            Then::Proxy {
                forward_uri,
                status,
            } => Ok(Uri::from_str(forward_uri)?),
            Then::Static { static_path } => Err(ConfigurationError::NotForwarding),
            Then::Mock { body, status } => Err(ConfigurationError::NotForwarding),
        };
        let request = Request::builder();
        Ok(request
            .uri(uri?)
            .method("GET")
            .body(Body::default())
            .unwrap())
    }
}

// convert to response
impl From<RuleAndIntermediaryHolder<'_>> for Response<Body> {
    fn from(holder: RuleAndIntermediaryHolder) -> Self {
        //TODO: apply transformations from rule

        let status = match &holder.rule.then {
            //plugins/transformation/status/headers
            Then::Fips {
                forward_uri,
                modify_response,
                status,
            } => {
                if let Some(modify) = modify_response {
                    match &modify.status {
                        Some(status) => hyper::StatusCode::from_str(&status).unwrap(),
                        None => hyper::StatusCode::OK,
                    }
                } else {
                    hyper::StatusCode::OK
                }
            }
            //plugins/headers/status
            Then::Mock { body, status } => {
                if let Some(status) = status {
                    hyper::StatusCode::from_str(&status).unwrap()
                } else {
                    hyper::StatusCode::OK
                }
            }
            //TODO body
            //headers
            Then::Proxy {
                forward_uri,
                status,
            } => todo!(),
            //nothing
            Then::Static { static_path } => todo!(),
        };

        let mut builder = holder
            .intermediary
            .headers
            .iter()
            .fold(Response::builder(), |builder, (key, value)| {
                builder.header(key, value)
            });

        for (key, value) in holder.intermediary.headers.iter() {
            builder = builder.header(key, value);
        }

        // CORS headers are always added to response
        builder = builder.header("Access-Control-Allow-Origin", "*");
        builder = builder.header("Access-Control-Allow-Methods", "*"); 

        let body = serde_json::to_string(&holder.intermediary.body).unwrap();
        let resp = Response::builder()
            .status(status)
            .body(Body::from(body))
            .unwrap();
        resp
    }
}
