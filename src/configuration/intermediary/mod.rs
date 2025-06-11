use std::future::Future;
use std::pin::Pin;
use bytes::Bytes;
use eyre::Result;
use hyper::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use http_body_util::{Full, Empty, BodyExt};
use http_body_util::combinators::BoxBody;
use hyper::{Request, Response};
use hyper::body::{Incoming};
use thiserror::Error;

// Define a trait for async conversions
pub trait AsyncConversion<T> {
    type Error;
    type Future: Future<Output = Result<T, Self::Error>>;
    fn try_convert(self) -> Self::Future;
}

#[derive(Debug, Error)]
pub enum IntermediaryError {
    #[error("Missing URI")]
    MissingUri,
    #[error("Missing method")]
    MissingMethod,
    #[error("Failed to create response: {0}")]
    ResponseCreation(String),
}

#[derive(Debug, Default, Clone)]
pub struct Intermediary {
    pub uri: Option<Uri>,
    pub method: Option<Method>,
    pub status: StatusCode,
    pub headers: HeaderMap<HeaderValue>,
    pub body: Option<String>,
}

impl Intermediary {
    pub fn new() -> Self {
        Self {
            uri: None,
            method: None,
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: None,
        }
    }
    
    pub async fn collect_body(mut body: Incoming) -> Result<String> {
        let mut body_bytes = vec![];
        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(|e| IntermediaryError::ResponseCreation(e.to_string()))?;
            if let Some(data) = frame.data_ref() {
                body_bytes.extend_from_slice(data);
            }
        }
        Ok(String::from_utf8_lossy(&body_bytes).into_owned())
    }
}

// Implementation for converting Request to Intermediary
impl AsyncConversion<Intermediary> for Request<IncomingBody> {
    type Error = IntermediaryError;
    type Future = Pin<Box<dyn Future<Output = Result<Intermediary, IntermediaryError>> + Send>>;

    fn try_convert(self) -> Self::Future {
        Box::pin(async move {
            let uri = self.uri().clone();
            let method = self.method().clone();
            let headers = self.headers().clone();
            let mut body_bytes = vec![];
            
            let mut body = self;
            while let Some(frame) = body.frame().await {
                let frame = frame.map_err(|e| IntermediaryError::ResponseCreation(e.to_string()))?;
                if let Some(data) = frame.data_ref() {
                    body_bytes.extend_from_slice(data);
                }
            }

            Ok(Intermediary {
                uri: Some(uri),
                method: Some(method),
                status: StatusCode::OK,
                headers,
                body: if body_bytes.is_empty() {
                    None
                } else {
                    Some(String::from_utf8_lossy(&body_bytes).into_owned())
                },
            })
        })
    }
}

// Implementation for converting Response to Intermediary
impl AsyncConversion<Intermediary> for Response<BoxedBody> {
    type Error = IntermediaryError;
    type Future = Pin<Box<dyn Future<Output = Result<Intermediary, IntermediaryError>> + Send>>;

    fn try_convert(self) -> Self::Future {
        Box::pin(async move {
            let status = self.status();
            let headers = self.headers().clone();
            let mut body_bytes = vec![];
            
            let mut body = self.into_body();
            while let Some(frame) = body.frame().await {
                let frame = frame.map_err(|e| IntermediaryError::ResponseCreation(e.to_string()))?;
                if let Some(data) = frame.data_ref() {
                    body_bytes.extend_from_slice(data);
                }
            }

            Ok(Intermediary {
                uri: None,
                method: None,
                status,
                headers,
                body: if body_bytes.is_empty() {
                    None
                } else {
                    Some(String::from_utf8_lossy(&body_bytes).into_owned())
                },
            })
        })
    }
}

// Helper function to convert Intermediary to Response
pub fn into_response(intermediary: Intermediary) -> Result<Response<BoxBody<Bytes, hyper::Error>>, IntermediaryError> {
    let mut response = Response::builder()
        .status(intermediary.status);

    // Add headers
    if !intermediary.headers.is_empty() {
        *response.headers_mut().unwrap() = intermediary.headers;
    }

    // Create the response body
    let body = if let Some(body_str) = intermediary.body {
        Full::new(Bytes::from(body_str))
            .map_err(|never| match never {})
            .boxed()
    } else {
        Empty::new()
            .map_err(|never| match never {})
            .boxed()
    };

    response.body(body)
        .map_err(|e| IntermediaryError::ResponseCreation(e.to_string()))
}

impl TryFrom<Request<Incoming>> for Intermediary {
    type Error = IntermediaryError;

    fn try_from(request: Request<Incoming>) -> Result<Self, Self::Error> {
        let (parts, _body) = request.into_parts();
        
        Ok(Self {
            uri: Some(parts.uri),
            method: Some(parts.method),
            status: StatusCode::OK,
            headers: parts.headers,
            body: None, // Body will be handled separately because it's async
        })
    }

}

// Body type aliases for clarity
pub type IncomingBody = Incoming;
pub type BoxedBody = BoxBody<Bytes, hyper::Error>;

// Helper functions for creating response bodies
pub fn empty_response_body() -> BoxedBody {
    Empty::new()
        .map_err(|never| match never {})
        .boxed()
}

pub fn string_response_body(content: String) -> BoxedBody {
    Full::new(Bytes::from(content))
        .map_err(|never| match never {})
        .boxed()
}

// Single conversion implementation from Intermediary to Response
impl TryFrom<Intermediary> for Response<BoxBody<Bytes, hyper::Error>> {
    type Error = IntermediaryError;

    fn try_from(intermediary: Intermediary) -> std::result::Result<Self, Self::Error> {
        let mut builder = Response::builder()
            .status(intermediary.status);

        // Add headers
        if !intermediary.headers.is_empty() {
            *builder.headers_mut().unwrap() = intermediary.headers;
        }

        // Create the response body
        let body = if let Some(body_str) = intermediary.body {
            Full::new(Bytes::from(body_str))
                .map_err(|never| match never {})
                .boxed()
        } else {
            Empty::new()
                .map_err(|never| match never {})
                .boxed()
        };

        builder.body(body)
            .map_err(|e| IntermediaryError::ResponseCreation(e.to_string()))
    }
}

// Helper functions for creating common responses
pub fn empty_response(status: StatusCode) -> Response<BoxedBody> {
    Response::builder()
        .status(status)
        .body(empty_response_body())
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(empty_response_body())
                .unwrap()
        })
}

pub fn string_response(content: String, status: StatusCode) -> Response<BoxedBody> {
    Response::builder()
        .status(status)
        .body(string_response_body(content))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(empty_response_body())
                .unwrap()
        })
}
