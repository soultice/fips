use bytes::Buf;
use http::{
    header::HeaderName, HeaderMap, HeaderValue, Method, StatusCode, Uri,
};
use hyper::{Body, Request, Response};
use json_dotpath::DotPaths;
use lazy_static::lazy_static;
use rand::Rng;
use regex::RegexSet;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf, str::FromStr, sync::Arc};
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

use crate::plugin_registry::ExternalFunctions;

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
pub struct ModifyResponseFips {
    #[serde(rename = "setHeaders")]
    add_headers: Option<HashMap<String, String>>,
    #[serde(rename = "keepHeaders")]
    delete_headers: Option<Vec<String>>,
    body: Option<Vec<BodyManipulation>>,
    status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModifyResponseProxy {
    #[serde(rename = "setHeaders")]
    add_headers: Option<HashMap<String, String>>,
    #[serde(rename = "keepHeaders")]
    delete_headers: Option<Vec<String>>,
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
    name: String,
    args: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct When {
    #[serde(rename = "matchesUris")]
    matches: Vec<Match>,
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
        modify_response: Option<ModifyResponseFips>,
    },
    Proxy {
        #[serde(rename = "forwardUri")]
        forward_uri: String,
        modify_response: Option<ModifyResponseProxy>,
    },
    Static {
        #[serde(rename = "baseDir")]
        static_base_dir: Option<String>,
    },
    Mock {
        body: Option<Value>,
        status: Option<String>,
        headers: Option<HashMap<String, String>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct With {
    pub sleep: Option<u64>,
    probability: Option<f32>,
    pub plugins: Option<Vec<Plugin>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum RuleSet {
    Rule(Rule),
}

impl RuleSet {
    pub fn into_inner(&self) -> &Rule {
        match self {
            RuleSet::Rule(rule) => rule,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Rule {
    pub name: String,
    pub when: When,
    pub then: Then,
    pub with: Option<With>,
    #[serde(skip)]
    pub path: String,
    #[serde(skip)]
    pub plugins: Option<ExternalFunctions>,
}

impl Rule {
    pub fn should_apply(&self, intermediary: &Intermediary) -> bool {
        let mut rng = rand::thread_rng();

        let uri_regex = RegexSet::new(
            self.when
                .matches
                .iter()
                .map(|m| m.uri.as_str())
                .collect::<Vec<&str>>(),
        )
        .unwrap();

        log::info!("uri_regex: {:?}", uri_regex);

        let uri = intermediary.clone().uri.unwrap();

        let some_uris_match = uri_regex.is_match(uri.path());
        log::info!("some_uris_match: {:?}", some_uris_match);
        if !some_uris_match {
            return false;
        }

        let some_methods_match =
            self.when.matches_methods.as_ref().map_or(true, |methods| {
                methods
                    .iter()
                    .any(|m| m == intermediary.clone().method.unwrap().as_str())
            });

        log::info!("some_methods_match: {:?}", some_methods_match);
        if !some_methods_match {
            return false;
        }

        let some_body_contains =
            self.when
                .body_contains
                .as_ref()
                .map_or(true, |body_contains| {
                    intermediary.body.as_str().unwrap().contains(body_contains)
                });
        log::info!("some_body_contains: {:?}", some_body_contains);
        if !some_body_contains {
            return false;
        }

        let probability_matches = self
            .with
            .as_ref()
            .unwrap_or(&With {
                probability: Some(1.0),
                plugins: None,
                sleep: None,
            })
            .probability
            .map(|probability| {
                let random_number = rng.gen_range(0.0, 0.99);
                random_number < probability
            })
            .unwrap_or(true);

        log::info!("probability_matches: {:?}", probability_matches);
        if !probability_matches {
            return false;
        }

        true
    }
}

#[derive(Deserialize, Clone, Debug, JsonSchema)]
pub struct NConfiguration {
    pub active_rule_indices: Vec<usize>,
    pub fe_selected_rule: usize,
    pub rules: Vec<RuleSet>,
}

impl Default for NConfiguration {
    fn default() -> Self {
        NConfiguration {
            active_rule_indices: vec![0],
            fe_selected_rule: 0,
            rules: vec![RuleSet::Rule(Rule {
                name: String::from("Static fallback - no rules found"),
                plugins: None,
                when: When {
                    matches: vec![Match {
                        uri: String::from(".*"),
                        body: None,
                    }],
                    matches_methods: None,
                    body_contains: None,
                },
                then: Then::Static {
                    static_base_dir: Some(
                        std::env::current_dir()
                            .unwrap()
                            .into_os_string()
                            .into_string()
                            .unwrap(),
                    ),
                },
                with: None,
                path: String::from(""),
            })],
        }
    }
}

impl NConfiguration {
    pub fn load(
        paths: &Vec<PathBuf>,
    ) -> Result<NConfiguration, DeserializationError> {
        let extensions = vec![String::from("yaml"), String::from("yml")];
        let loader = YamlFileLoader { extensions };
        let mut rules = loader.load_from_directories(paths)?;

        //load plugins
        for rule in &mut rules {
            match rule {
                RuleSet::Rule(rule) => {
                    if let Some(with) = &rule.with {
                        if let Some(plugins) = &with.plugins {
                            for plugin in plugins {
                                let path = PathBuf::from(&plugin.path);
                                let absolute_path = path.canonicalize()?;
                                let external_functions =
                                    ExternalFunctions::new(&absolute_path);
                                rule.plugins = Some(external_functions);
                            }
                        }
                    }
                }
            }
        }

        log::info!("Loaded rules: {:?}", rules);

        Ok(NConfiguration {
            //all rules are active initially
            active_rule_indices: (0..rules.len()).collect(),
            fe_selected_rule: 0,
            rules,
        })
    }

    pub fn reload(&mut self, paths: &Vec<PathBuf>) -> Result<(), String> {
        //TODO enable plugin reload
        match NConfiguration::load(paths) {
            Ok(new_config) => {
                self.rules = new_config.rules;
                self.active_rule_indices = new_config.active_rule_indices;
                self.fe_selected_rule = new_config.fe_selected_rule;
                Ok(())
            }
            Err(e) => Err(format!("Error reloading config: {e:?}")),
        }
    }

    pub fn select_next(&mut self) {
        self.fe_selected_rule = (self.fe_selected_rule + 1) % self.rules.len();
    }

    pub fn select_previous(&mut self) {
        self.fe_selected_rule =
            (self.fe_selected_rule + self.rules.len() - 1) % self.rules.len();
    }

    pub fn toggle_rule(&mut self) {
        log::info!("Toggling rule: {}", self.fe_selected_rule);
        if self.active_rule_indices.contains(&self.fe_selected_rule) {
            self.remove_from_active_indices();
            log::info!(
                "Removed rule: {}, {:?}",
                self.fe_selected_rule,
                self.active_rule_indices
            );
        } else {
            self.add_to_active_indices();
            log::info!(
                "Removed rule: {}, {:?}",
                self.fe_selected_rule,
                self.active_rule_indices
            );
        }
    }

    pub fn remove_from_active_indices(&mut self) {
        self.active_rule_indices
            .retain(|&x| x != self.fe_selected_rule);
    }

    pub fn add_to_active_indices(&mut self) {
        self.active_rule_indices.push(self.fe_selected_rule);
    }
}

#[derive(Debug, Clone)]
pub struct Intermediary {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: serde_json::Value,
    pub method: Option<Method>,
    pub uri: Option<Uri>,
}

pub trait AsyncFrom<T> {
    type Output;
    async fn async_from(t: T) -> Self::Output;
}

impl AsyncFrom<hyper::Response<hyper::Body>> for Intermediary {
    type Output = Intermediary;

    async fn async_from(
        response: hyper::Response<hyper::Body>,
    ) -> Intermediary {
        let status = response.status();
        let mut headers = response.headers().clone();
        headers.remove("content-length");

        let body = response.into_body();
        let body = hyper::body::aggregate(body).await.unwrap().reader();
        let resp_json: serde_json::Value =
            serde_json::from_reader(body).unwrap_or_default();
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
        let uri = request.uri().clone();
        let headers = request.headers().clone();
        let body = request.into_body();
        let body = hyper::body::aggregate(body).await.unwrap().reader();
        let req_json: serde_json::Value =
            serde_json::from_reader(body).unwrap_or_default();
        Intermediary {
            status: StatusCode::OK,
            headers,
            body: req_json,
            method: Some(method),
            uri: Some(uri),
        }
    }
}

impl From<Intermediary> for hyper::Response<hyper::Body> {
    fn from(intermediary: Intermediary) -> Self {
        let mut builder = Response::builder();
        builder = builder.status(intermediary.status);
        for (key, value) in intermediary.headers.iter() {
            builder = builder.header(key, value);
        }
        let body = serde_json::to_string(&intermediary.body).unwrap();
        builder.body(Body::from(body)).unwrap()
    }
}

impl TryFrom<Intermediary> for hyper::Request<hyper::Body> {
    type Error = ConfigurationError;
    fn try_from(
        intermediary: Intermediary,
    ) -> Result<Self, ConfigurationError> {
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
    pub intermediary: Intermediary,
}

impl RuleAndIntermediaryHolder<'_> {
    pub fn apply_plugins(&self, next: &mut serde_json::Value) {
        if let Some(plugins) = &self.rule.plugins {
            log::info!("Plugins: {:?}, next: {:?}", plugins, next);
            match next {
                serde_json::Value::String(val) => {
                    if plugins.has(&val) {
                        let result = plugins
                            .call(&val, vec![])
                            .expect("Invocation failed");
                        let try_serialize = serde_json::from_str(&result);
                        if let Ok(i) = try_serialize {
                            *next = i;
                        } else {
                            *next = serde_json::Value::String(result);
                        }
                    }
                }
                serde_json::Value::Array(val) => {
                    for i in val {
                        self.apply_plugins(i);
                    }
                }
                serde_json::Value::Object(val) => {
                    let plugin = val.get("plugin");
                    let args = val.get("args");
                    match (plugin, args) {
                        (Some(p), Some(a)) => {
                            if let (
                                serde_json::Value::String(function),
                                serde_json::Value::Array(arguments),
                            ) = (p, a)
                            {
                                let result = plugins
                                    .call(function, arguments.clone())
                                    .expect("Invocation failed");
                                let try_serialize =
                                    serde_json::from_str(&result);
                                if let Ok(i) = try_serialize {
                                    *next = i;
                                } else {
                                    *next = serde_json::Value::String(result);
                                }
                            }
                        }
                        _ => {
                            for (_, i) in val {
                                self.apply_plugins(i);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

//convert from holder to hyper request
impl TryFrom<&RuleAndIntermediaryHolder<'_>> for Request<Body> {
    type Error = ConfigurationError;

    fn try_from(
        holder: &RuleAndIntermediaryHolder,
    ) -> Result<Self, ConfigurationError> {
        let mut builder = holder
            .intermediary
            .headers
            .iter()
            .fold(Request::builder(), |builder, (key, value)| {
                builder.header(key, value)
            });
        match &holder.rule.then {
            Then::Fips {
                forward_uri,
                modify_response: _,
            } => {
                builder = builder.uri(Uri::from_str(forward_uri)?);
            }
            Then::Proxy {
                forward_uri,
                modify_response: _,
            } => {
                builder = builder.uri(Uri::from_str(forward_uri)?);
            }
            Then::Static { static_base_dir: _ } => {
                return Err(ConfigurationError::NotForwarding);
            }
            Then::Mock {
                body: _,
                status: _,
                headers: _,
            } => return Err(ConfigurationError::NotForwarding),
        };
        builder = builder.method(holder.intermediary.method.clone().unwrap());
        Ok(builder
            .body(Body::from(holder.intermediary.body.to_string()))
            .unwrap())
    }
}

// convert to response
impl AsyncFrom<RuleAndIntermediaryHolder<'_>> for Response<Body> {
    type Output = Response<Body>;
    async fn async_from(holder: RuleAndIntermediaryHolder<'_>) -> Self {
        let preemtive_body = &mut holder.intermediary.body.clone();

        let mut builder = holder
            .intermediary
            .headers
            .iter()
            .fold(Response::builder(), |builder, (key, value)| {
                builder.header(key, value)
            });

        log::info!("Response from Intermediary");

        match &holder.rule.then {
            //plugins/transformation/status/headers
            //TODO plugins
            Then::Fips {
                forward_uri: _,
                modify_response,
            } => {
                log::info!("fips rule {:?}", modify_response);
                if let Some(modify) = modify_response {
                    if let Some(status) = &modify.status {
                        builder = builder.status(
                            hyper::StatusCode::from_str(status).unwrap(),
                        );
                    }

                    //morph body
                    if let Some(manipulator) = &modify.body {
                        let mut body = holder.intermediary.body.clone();
                        manipulator.iter().for_each(|m| {
                            body.dot_set(&m.at, &m.with).unwrap();
                        });
                    }

                    if let Some(headers) = &modify.delete_headers {
                        let headers_mut = builder.headers_mut().unwrap();
                        for h in headers {
                            if headers_mut.contains_key(h) {
                                headers_mut.remove(h);
                            }
                        }
                    }

                    if let Some(add_headers) = &modify.add_headers {
                        for (key, value) in add_headers.iter() {
                            builder = builder.header(key, value);
                        }
                    }
                }
            }
            //plugins/headers/status
            Then::Mock {
                body,
                status,
                headers: _,
            } => {
                log::info!("mock rule {:?}", body);
                if let Some(status) = status {
                    builder = builder
                        .status(hyper::StatusCode::from_str(status).unwrap());
                } else {
                    builder = builder.status(hyper::StatusCode::OK)
                }
                if let Some(body) = body {
                    *preemtive_body = body.clone();
                }
            }
            //headers
            Then::Proxy {
                forward_uri: _,
                modify_response,
            } => {
                log::info!("proxy rule {:?}", modify_response);
                if let Some(modify_response) = modify_response {
                    if let Some(status) = &modify_response.status {
                        builder = builder.status(
                            hyper::StatusCode::from_str(status).unwrap(),
                        );
                    }
                    if let Some(add_headers) = &modify_response.add_headers {
                        for (key, value) in add_headers.iter() {
                            builder = builder.header(key, value);
                        }
                    }
                    if let Some(delete_headers) =
                        &modify_response.delete_headers
                    {
                        for h in delete_headers {
                            if builder.headers_mut().unwrap().contains_key(h) {
                                builder.headers_mut().unwrap().remove(h);
                            }
                        }
                    }
                }
            }
            //nothing
            Then::Static { static_base_dir } => {
                log::info!(
                    "static base dir: {:?}, uri: {}",
                    static_base_dir,
                    holder.intermediary.uri.as_ref().unwrap()
                );
                if let Some(path) = static_base_dir {
                    let static_path = hyper_staticfile::resolve_path(
                        path,
                        holder.intermediary.uri.clone().unwrap().path(),
                    )
                    .await
                    .unwrap();

                    let header_name = HeaderName::from_static("x-static");
                    let header_value = HeaderValue::from_str(path).unwrap();
                    let resp = hyper_staticfile::ResponseBuilder::new()
                        .request_parts(
                            holder.intermediary.method.as_ref().unwrap(),
                            &holder.intermediary.uri.clone().unwrap(),
                            &HeaderMap::from_iter(vec![(
                                header_name,
                                header_value,
                            )]),
                        )
                        .build(static_path)
                        .unwrap();
                    return resp;
                }
            }
        };

        // CORS headers are always added to response
        builder = builder.header("Access-Control-Allow-Origin", "*");
        builder = builder.header("Access-Control-Allow-Methods", "*");

        holder.apply_plugins(preemtive_body);
        log::info!("after plugins: {:?}", preemtive_body);
        let resp_body = Body::from(preemtive_body.to_string());
        let resp = builder.body(resp_body).unwrap();
        log::info!("response: {:?}", resp);
        resp
    }
}
