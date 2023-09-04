use bytes::Buf;
use eyre::{Context, ContextCompat, Result, eyre};
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
    #[error("Plugin invocation error: {0}")]
    PluginInvocation(#[from] InvocationError),
    #[error("Plugin not found error")]
    PluginNotFound,
    #[error("could not parse yaml")]
    YamlParse(#[from] serde_yaml::Error),
    #[error("could not parse json")]
    JsonParse(#[from] serde_json::Error),
    #[error("std error")]
    Std(#[from] std::io::Error),
    #[error("hyper lib error")]
    Hyper(#[from] hyper::Error),
    #[error("rule does not match")]
    RuleDoesNotMatch,
}

use crate::plugin_registry::{ExternalFunctions, InvocationError};

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
    set_headers: Option<HashMap<String, String>>,
    #[serde(rename = "deleteHeaders")]
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
    with: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Plugin {
    path: String,
    name: String,
    args: Option<Value>,
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
    pub fn should_apply(&self, intermediary: &Intermediary) -> Result<()> {
        let mut rng = rand::thread_rng();

        let uri_regex = RegexSet::new(
            self.when
                .matches
                .iter()
                .map(|m| m.uri.as_str())
                .collect::<Vec<&str>>(),
        )?;

        let uri = intermediary.clone().uri.wrap_err("could not retrieve uri")?;

        let some_uris_match = uri_regex.is_match(uri.path());
        if !some_uris_match {
            return Err(ConfigurationError::RuleDoesNotMatch.into());
        }

        let some_methods_match =
            self.when.matches_methods.as_ref().map_or(true, |methods| {
                methods
                    .iter()
                    .any(|m| m == intermediary.clone().method.unwrap().as_str())
            });

        if !some_methods_match {
            return Err(ConfigurationError::RuleDoesNotMatch.into());
        }

        let some_body_contains =
            self.when
                .body_contains
                .as_ref()
                .map_or(true, |body_contains| {
                    intermediary.body.as_str().unwrap().contains(body_contains)
                });

        if !some_body_contains {
            return Err(ConfigurationError::RuleDoesNotMatch.into());
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

        if !probability_matches {
            return Err(ConfigurationError::RuleDoesNotMatch.into());
        }

        Ok(())
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
        paths: &[PathBuf],
    ) -> Result<NConfiguration, DeserializationError> {
        let extensions = vec![String::from("yaml"), String::from("yml")];
        let loader = YamlFileLoader { extensions };
        let mut rules = loader.load_from_directories(paths)?;

        if rules.is_empty() {
            return Ok(NConfiguration::default())
        }

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

        Ok(NConfiguration {
            //all rules are active initially
            active_rule_indices: (0..rules.len()).collect(),
            fe_selected_rule: 0,
            rules,
        })
    }

    pub fn reload(&mut self, paths: &[PathBuf]) -> Result<()> {
        //TODO enable plugin reload
        match NConfiguration::load(paths) {
            Ok(new_config) => {
                self.rules = new_config.rules;
                self.active_rule_indices = new_config.active_rule_indices;
                self.fe_selected_rule = new_config.fe_selected_rule;
                Ok(())
            }
            Err(e) => Err(eyre!("Error reloading config: {e:?}")),
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
        if self.active_rule_indices.contains(&self.fe_selected_rule) {
            self.remove_from_active_indices();
        } else {
            self.add_to_active_indices();
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

pub trait AsyncTryFrom<T> {
    type Output;
    async fn async_try_from(t: T) -> Result<Self::Output>;
}

impl AsyncTryFrom<hyper::Response<hyper::Body>> for Intermediary {
    type Output = Intermediary;

    async fn async_try_from(
        response: hyper::Response<hyper::Body>,
    ) -> Result<Intermediary> {
        let status = response.status();
        let mut headers = response.headers().clone();
        headers.remove("content-length");

        let body = response.into_body();
        let body = hyper::body::aggregate(body).await?.reader();
        let resp_json: serde_json::Value =
            serde_json::from_reader(body).unwrap_or_default();
        Ok(Intermediary {
            status,
            headers,
            body: resp_json,
            method: None,
            uri: None,
        })
    }
}

impl AsyncTryFrom<hyper::Request<hyper::Body>> for Intermediary {
    type Output = Intermediary;
    async fn async_try_from(
        request: hyper::Request<hyper::Body>,
    ) -> Result<Intermediary> {
        let method = request.method().clone();
        let uri = request.uri().clone();
        let headers = request.headers().clone();
        let body = request.into_body();
        let body = hyper::body::aggregate(body).await?.reader();
        let req_json: serde_json::Value =
            serde_json::from_reader(body).unwrap_or_default();
        Ok(Intermediary {
            status: StatusCode::OK,
            headers,
            body: req_json,
            method: Some(method),
            uri: Some(uri),
        })
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
    pub fn apply_plugins(
        &self,
        next: &mut serde_json::Value,
    ) -> Result<(), ConfigurationError> {
        if let Some(plugins) = &self.rule.plugins {
            match next {
                serde_json::Value::String(plugin_name) => {
                    if plugins.has(plugin_name) {
                        let rule_plugins = &self
                            .rule
                            .clone()
                            .with
                            .ok_or(ConfigurationError::PluginNotFound)?
                            .plugins
                            .ok_or(ConfigurationError::PluginNotFound)?;

                        let plugin_config = rule_plugins
                            .iter()
                            .find(|p| p.name == *plugin_name)
                            .ok_or(ConfigurationError::PluginNotFound)?;

                        let plugin_args =
                            plugin_config.args.clone().unwrap_or_default();

                        let result = plugins.call(plugin_name, plugin_args)?;

                        // try to deserialize, if it fails, parse it into a string
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
                        self.apply_plugins(i)?;
                    }
                }
                serde_json::Value::Object(val) => {
                    let plugin = val.get("plugin");
                    let args = val.get("args");
                    match (plugin, args) {
                        (Some(p), Some(a)) => {
                            if let (
                                serde_json::Value::String(function),
                                arguments,
                            ) = (p, a)
                            {
                                let result = plugins
                                    .call(function, arguments.clone())?;
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
                                self.apply_plugins(i)?;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
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
        builder = builder.method(
            holder
                .intermediary
                .method
                .clone()
                .ok_or(ConfigurationError::NoMethodError)?,
        );
        Ok(builder.body(Body::from(holder.intermediary.body.to_string()))?)
    }
}

// convert to response
impl AsyncTryFrom<RuleAndIntermediaryHolder<'_>> for Response<Body> {
    type Output = Response<Body>;
    async fn async_try_from(
        holder: RuleAndIntermediaryHolder<'_>,
    ) -> Result<Self> {
        let preemtive_body = &mut holder.intermediary.body.clone();
        let preemtive_header_map = &mut holder.intermediary.headers.clone();

        let mut builder = holder
            .intermediary
            .headers
            .iter()
            .fold(Response::builder(), |builder, (key, value)| {
                builder.header(key, value)
            });

        builder.headers_mut().wrap_err("hyper object has no headers")?.remove("content-length");
        preemtive_header_map.remove(HeaderName::from_static("content-length"));

        match &holder.rule.then {
            //plugins/transformation/status/headers
            //TODO plugins
            Then::Fips {
                forward_uri: _,
                modify_response,
            } => {
                if let Some(modify) = modify_response {
                    if let Some(status) = &modify.status {
                        builder = builder
                            .status(hyper::StatusCode::from_str(status)?);
                    }

                    //morph body
                    if let Some(manipulator) = &modify.body {
                        manipulator.iter().for_each(|m| {
                            preemtive_body
                                .dot_set(&m.at, &m.with)
                                .wrap_err("invalid 'at' in rule").unwrap();
                        });
                    }

                    if let Some(headers) = &modify.delete_headers {
                        for h in headers {
                            if preemtive_header_map.contains_key(h) {
                                preemtive_header_map.remove(h);
                            }
                        }
                    }

                    if let Some(add_headers) = &modify.set_headers {
                        for (key, value) in add_headers.iter() {
                            if preemtive_header_map.contains_key(key) {
                                preemtive_header_map.remove(key);
                            }
                            builder = builder.header(key, value);
                        }
                    }
                }
            }
            //plugins/headers/status
            Then::Mock {
                body,
                status,
                headers,
            } => {
                if let Some(status) = status {
                    builder =
                        builder.status(hyper::StatusCode::from_str(status)?);
                } else {
                    builder = builder.status(hyper::StatusCode::OK)
                }
                if let Some(body) = body {
                    *preemtive_body = body.clone();
                }
                if let Some(headers) = headers {
                    for (key, value) in headers.iter() {
                        preemtive_header_map.insert(
                            HeaderName::from_str(key)?,
                            HeaderValue::from_str(value)?,
                        );
                    }
                }
            }
            //headers
            Then::Proxy {
                forward_uri: _,
                modify_response,
            } => {
                if let Some(modify_response) = modify_response {
                    if let Some(status) = &modify_response.status {
                        builder = builder
                            .status(hyper::StatusCode::from_str(status)?);
                    }
                    if let Some(add_headers) = &modify_response.add_headers {
                        for (key, value) in add_headers.iter() {
                            preemtive_header_map.insert(
                                HeaderName::from_str(key)?,
                                HeaderValue::from_str(value)?,
                            );
                        }
                    }
                    if let Some(delete_headers) =
                        &modify_response.delete_headers
                    {
                        for h in delete_headers {
                            if preemtive_header_map.contains_key(h) {
                                preemtive_header_map.remove(h);
                            }
                        }
                    }
                }
            }
            //nothing
            Then::Static { static_base_dir } => {
                if let Some(path) = static_base_dir {
                    let static_path = hyper_staticfile::resolve_path(
                        path,
                        holder
                            .intermediary
                            .uri
                            .clone()
                            .wrap_err("could not retrieve uri")?
                            .path(),
                    )
                    .await
                    .unwrap();

                    let header_name = HeaderName::from_static("x-static");
                    let header_value = HeaderValue::from_str(path)?;
                    let resp = hyper_staticfile::ResponseBuilder::new()
                        .request_parts(
                            holder.intermediary.method.as_ref().wrap_err("could not retrieve method")?,
                            &holder.intermediary.uri.clone().wrap_err("could not retrieve uri")?,
                            &HeaderMap::from_iter(vec![(
                                header_name,
                                header_value,
                            )]),
                        )
                        .build(static_path)?;
                    return Ok(resp);
                }
            }
        };

        // CORS headers are always added to preflight response
        // FIXME: check if headers are already present, if so overwrite them
        log::info!("method: {:?}", holder.intermediary.method);
        if holder.intermediary.method.as_ref() == Some(&Method::OPTIONS) {
            preemtive_header_map.insert(
                HeaderName::from_static("Access-Control-Allow-Origin"),
                HeaderValue::from_static("*"),
            );
            preemtive_header_map.insert(
                HeaderName::from_static("Access-Control-Allow-Methods"),
                HeaderValue::from_static("*"),
            );
            preemtive_header_map.insert(
                HeaderName::from_static("Access-Control-Allow-Headers"),
                HeaderValue::from_static("*"),
            );
            preemtive_header_map.insert(
                HeaderName::from_static("Access-Control-Max-Age"),
                HeaderValue::from_static("86400"),
            );
        }

        holder.apply_plugins(preemtive_body)?;

        let resp_body = Body::from(preemtive_body.to_string());

        //flush the header map
        builder
            .headers_mut()
            .wrap_err("hyper object has no headers")?
            .clear();


        builder
            .headers_mut()
            .wrap_err("hyper object has no headers")?
            .extend(preemtive_header_map.clone());

        Ok(builder.body(resp_body)?)
    }
}
