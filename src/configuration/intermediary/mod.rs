use bytes::Bytes;
use eyre::Result;
use http::{HeaderMap, Method, StatusCode, Uri};
use hyper::{Request, Response};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;

use super::rule::error::ConfigurationError;

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

impl AsyncTryFrom<hyper::Response<Incoming>> for Intermediary {
    type Output = Intermediary;

    async fn async_try_from(
        response: hyper::Response<Incoming>,
    ) -> Result<Intermediary> {
        let status = response.status();
        let mut headers = response.headers().clone();
        headers.remove("content-length");

        let body = response.into_body();
        let body_bytes = body.collect().await?.to_bytes();
        let resp_json: serde_json::Value =
            serde_json::from_slice(&body_bytes).unwrap_or_default();
        Ok(Intermediary {
            status,
            headers,
            body: resp_json,
            method: None,
            uri: None,
        })
    }
}

impl AsyncTryFrom<hyper::Request<Incoming>> for Intermediary {
    type Output = Intermediary;
    async fn async_try_from(
        request: hyper::Request<Incoming>,
    ) -> Result<Intermediary> {
        let method = request.method().clone();
        let uri = request.uri().clone();
        let headers = request.headers().clone();
        let body = request.into_body();
        let body_bytes = body.collect().await?.to_bytes();
        let req_json: serde_json::Value =
            serde_json::from_slice(&body_bytes).unwrap_or_default();
        Ok(Intermediary {
            status: StatusCode::OK,
            headers,
            body: req_json,
            method: Some(method),
            uri: Some(uri),
        })
    }
}

impl From<Intermediary> for hyper::Response<Full<Bytes>> {
    fn from(intermediary: Intermediary) -> Self {
        let mut builder = Response::builder();
        builder = builder.status(intermediary.status);
        for (key, value) in intermediary.headers.iter() {
            builder = builder.header(key, value);
        }
        let body = serde_json::to_string(&intermediary.body).unwrap();
        builder.body(Full::new(Bytes::from(body))).unwrap()
    }
}

impl TryFrom<Intermediary> for hyper::Request<Full<Bytes>> {
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
        Ok(builder.body(Full::new(Bytes::from(intermediary.body.to_string())))?)
    }
}
