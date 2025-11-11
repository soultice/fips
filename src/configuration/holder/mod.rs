use std::str::FromStr;

use bytes::Bytes;
use http::{
    header::HeaderName, HeaderValue, Method, Uri,
};
use hyper::{Request, Response};
use http_body_util::{Full, BodyExt};
use json_dotpath::DotPaths;

use super::rule::{ Rule, error::ConfigurationError, then::Then} ;
use super::intermediary::{AsyncTryFrom, Intermediary};

use eyre::{Context, ContextCompat, Result};


pub struct RuleAndIntermediaryHolder {
    pub rule: Rule,
    pub intermediary: Intermediary,
    #[cfg(feature = "logging")]
    pub correlation_id: u64,
}

impl RuleAndIntermediaryHolder {
    fn apply_plugins_to_body(
        rule: &Rule,
        plugins: &crate::plugin_registry::ExternalFunctions,
        next: &mut serde_json::Value,
    ) -> Result<(), ConfigurationError> {
        {
            match next {
                serde_json::Value::String(plugin_name) => {
                    // Strip {{ and }} from the plugin placeholder
                    let stripped_name = plugin_name
                        .strip_prefix("{{")
                        .and_then(|s| s.strip_suffix("}}"))
                        .unwrap_or(plugin_name);
                    
                    if plugins.has(stripped_name) {
                        let rule_plugins = rule
                            .with
                            .as_ref()
                            .and_then(|w| w.plugins.as_ref())
                            .ok_or(ConfigurationError::PluginNotFound)?;

                        let plugin_config = rule_plugins
                            .iter()
                            .find(|p| p.name == stripped_name)
                            .ok_or(ConfigurationError::PluginNotFound)?;

                        let plugin_args = plugin_config.args.as_ref()
                            .cloned()
                            .unwrap_or_default();

                        let result = plugins.call(stripped_name, plugin_args)?;

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
                        Self::apply_plugins_to_body(rule, plugins, i)?;
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
                                Self::apply_plugins_to_body(rule, plugins, i)?;
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
impl TryFrom<&RuleAndIntermediaryHolder> for Request<Full<Bytes>> {
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
                #[cfg(feature = "logging")]
                log::info!("[cid={}] [holder] building forwarding request THEN=Fips forward_uri='{}' rule='{}'", holder.correlation_id, forward_uri, holder.rule.name);
                builder = builder.uri(Uri::from_str(forward_uri)?);
            }
            Then::Proxy {
                forward_uri,
                modify_response: _,
            } => {
                #[cfg(feature = "logging")]
                log::info!("[cid={}] [holder] building forwarding request THEN=Proxy forward_uri='{}' rule='{}'", holder.correlation_id, forward_uri, holder.rule.name);
                builder = builder.uri(Uri::from_str(forward_uri)?);
            }
            Then::Static { static_base_dir: _, strip_path: _ } => {
                #[cfg(feature = "logging")]
                log::info!("[cid={}] [holder] request conversion aborted THEN=Static rule='{}' (not forwarding)", holder.correlation_id, holder.rule.name);
                return Err(ConfigurationError::NotForwarding);
            }
            Then::Mock {
                body: _,
                status: _,
                headers: _,
            } => {
                #[cfg(feature = "logging")]
                log::info!("[cid={}] [holder] request conversion aborted THEN=Mock rule='{}' (not forwarding)", holder.correlation_id, holder.rule.name);
                return Err(ConfigurationError::NotForwarding)
            },
        };
        builder = builder.method(
            holder
                .intermediary
                .method
                .as_ref()
                .ok_or(ConfigurationError::NoMethodError)?
                .clone(),
        );
        Ok(builder.body(Full::new(Bytes::from(holder.intermediary.body.to_string())))?)
    }
}

// convert to response
impl AsyncTryFrom<RuleAndIntermediaryHolder> for Response<Full<Bytes>> {
    type Output = Response<Full<Bytes>>;
    async fn async_try_from(
        mut holder: RuleAndIntermediaryHolder,
    ) -> Result<Self> {
        let mut builder = holder
            .intermediary
            .headers
            .iter()
            .fold(Response::builder(), |builder, (key, value)| {
                builder.header(key, value)
            });

        builder.headers_mut().wrap_err("hyper object has no headers")?.remove("content-length");
        holder.intermediary.headers.remove(HeaderName::from_static("content-length"));

        match &holder.rule.then {
            //plugins/transformation/status/headers
            //TODO plugins
            Then::Fips {
                forward_uri: _,
                modify_response,
            } => {
                #[cfg(feature = "logging")]
                log::info!("[cid={}] [holder] building response THEN=Fips rule='{}'", holder.correlation_id, holder.rule.name);
                if let Some(modify) = modify_response {
                    if let Some(status) = &modify.status {
                        builder = builder
                            .status(hyper::StatusCode::from_str(status)?);
                    }

                    //morph body
                    if let Some(manipulator) = &modify.body {
                        manipulator.iter().for_each(|m| {
                            holder.intermediary.body
                                .dot_set(&m.at, &m.with)
                                .wrap_err("invalid 'at' in rule").unwrap();
                        });
                    }

                    if let Some(headers) = &modify.delete_headers {
                        for h in headers {
                            if holder.intermediary.headers.contains_key(h) {
                                holder.intermediary.headers.remove(h);
                            }
                        }
                    }

                    if let Some(add_headers) = &modify.set_headers {
                        for (key, value) in add_headers.iter() {
                            if holder.intermediary.headers.contains_key(key) {
                                holder.intermediary.headers.remove(key);
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
                #[cfg(feature = "logging")]
                log::info!("[cid={}] [holder] building response THEN=Mock rule='{}' status='{:?}'", holder.correlation_id, holder.rule.name, status);
                if let Some(status) = status {
                    builder =
                        builder.status(hyper::StatusCode::from_str(status)?);
                } else {
                    builder = builder.status(hyper::StatusCode::OK)
                }
                if let Some(body) = body {
                    holder.intermediary.body = body.clone();
                }
                if let Some(headers) = headers {
                    for (key, value) in headers.iter() {
                        holder.intermediary.headers.insert(
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
                #[cfg(feature = "logging")]
                log::info!("[cid={}] [holder] building response THEN=Proxy rule='{}'", holder.correlation_id, holder.rule.name);
                if let Some(modify_response) = modify_response {
                    if let Some(status) = &modify_response.status {
                        builder = builder
                            .status(hyper::StatusCode::from_str(status)?);
                    }
                    if let Some(add_headers) = &modify_response.add_headers {
                        for (key, value) in add_headers.iter() {
                            holder.intermediary.headers.insert(
                                HeaderName::from_str(key)?,
                                HeaderValue::from_str(value)?,
                            );
                        }
                    }
                    if let Some(delete_headers) =
                        &modify_response.delete_headers
                    {
                        for h in delete_headers {
                            if holder.intermediary.headers.contains_key(h) {
                                holder.intermediary.headers.remove(h);
                            }
                        }
                    }
                }
            }
            //nothing
            Then::Static { static_base_dir, strip_path } => {
                if let Some(path) = static_base_dir {
                    #[cfg(feature = "logging")]
                    log::info!("[cid={}] [holder] serving static THEN=Static rule='{}' base_dir='{}' strip_path={} ", holder.correlation_id, holder.rule.name, path, strip_path);
                    // Build a fake request to use with hyper-staticfile
                    let original_uri = holder.intermediary.uri.as_ref().wrap_err("could not retrieve uri")?;
                    let effective_uri = if *strip_path {
                        // Extract just the final segment (filename) from the path
                        let path_str = original_uri.path();
                        let filename = path_str.rsplit('/').next().unwrap_or(path_str);
                        // Reconstruct URI with leading slash and filename only, preserving query if any
                        let mut new_path = String::from("/");
                        new_path.push_str(filename);
                        let rebuilt = if let Some(query) = original_uri.query() {
                            format!("{}?{}", new_path, query)
                        } else {
                            new_path
                        };
                        #[cfg(feature = "logging")]
                        log::debug!("[cid={}] [holder] static strip applied original='{}' effective='{}'", holder.correlation_id, original_uri, rebuilt);
                        Uri::from_str(&rebuilt)?
                    } else {
                        original_uri.clone()
                    };
                    let fake_request = Request::builder()
                        .method(holder.intermediary.method.as_ref().wrap_err("could not retrieve method")?)
                        .uri(effective_uri)
                        .body(Full::new(Bytes::new()))?;
                    
                    let resp = hyper_staticfile::Static::new(std::path::Path::new(path))
                        .serve(fake_request)
                        .await?;
                    
                    // Convert the hyper_staticfile response to our Response<Full<Bytes>> type
                    let (parts, body) = resp.into_parts();
                    let body_bytes = body.collect().await?.to_bytes();
                    let converted_resp = Response::from_parts(parts, Full::new(body_bytes));
                    return Ok(converted_resp);
                }
            }
        };

        // CORS headers are always added to preflight response
        // FIXME: check if headers are already present, if so overwrite them
        log::info!("method: {:?}", holder.intermediary.method);
        if holder.intermediary.method.as_ref() == Some(&Method::OPTIONS) {
            holder.intermediary.headers.insert(
                HeaderName::from_static("Access-Control-Allow-Origin"),
                HeaderValue::from_static("*"),
            );
            holder.intermediary.headers.insert(
                HeaderName::from_static("Access-Control-Allow-Methods"),
                HeaderValue::from_static("*"),
            );
            holder.intermediary.headers.insert(
                HeaderName::from_static("Access-Control-Allow-Headers"),
                HeaderValue::from_static("*"),
            );
            holder.intermediary.headers.insert(
                HeaderName::from_static("Access-Control-Max-Age"),
                HeaderValue::from_static("86400"),
            );
        }

        // Apply plugins - need to extract body temporarily to satisfy borrow checker
        {
            let body = &mut holder.intermediary.body;
            if let Some(plugins) = &holder.rule.plugins {
                RuleAndIntermediaryHolder::apply_plugins_to_body(&holder.rule, plugins, body)?;
            }
        }

        let resp_body = Full::new(Bytes::from(holder.intermediary.body.to_string()));

        //flush the header map
        builder
            .headers_mut()
            .wrap_err("hyper object has no headers")?
            .clear();


        builder
            .headers_mut()
            .wrap_err("hyper object has no headers")?
            .extend(std::mem::take(&mut holder.intermediary.headers));

        Ok(builder.body(resp_body)?)
    }
}
