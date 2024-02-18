use std::str::FromStr;

use http::{
    header::HeaderName, HeaderMap, HeaderValue, Method, Uri,
};
use hyper::{Body, Request, Response};
use json_dotpath::DotPaths;

use super::rule::{ Rule, error::ConfigurationError, then::Then} ;
use super::intermediary::{AsyncTryFrom, Intermediary};

use eyre::{Context, ContextCompat, Result};


pub struct RuleAndIntermediaryHolder {
    pub rule: Rule,
    pub intermediary: Intermediary,
}

impl RuleAndIntermediaryHolder {
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
impl TryFrom<&RuleAndIntermediaryHolder> for Request<Body> {
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
impl AsyncTryFrom<RuleAndIntermediaryHolder> for Response<Body> {
    type Output = Response<Body>;
    async fn async_try_from(
        holder: RuleAndIntermediaryHolder,
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
