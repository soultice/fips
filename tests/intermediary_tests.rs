//! Tests for intermediary conversions

mod common;

use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode, Uri};
use http_body_util::Full;
use hyper::{Request, Response};
use serde_json::json;

#[test]
fn test_intermediary_to_response_conversion() {
    let mut headers = HeaderMap::new();
    headers.insert("x-custom", "test-value".parse().unwrap());
    
    let intermediary = fips::configuration::intermediary::Intermediary {
        status: StatusCode::OK,
        headers,
        body: json!({"message": "success"}),
        method: None,
        uri: None,
    };
    
    let response: Response<Full<Bytes>> = intermediary.into();
    
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("x-custom").unwrap(),
        "test-value"
    );
}

#[test]
fn test_intermediary_to_request_conversion() {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    
    let intermediary = fips::configuration::intermediary::Intermediary {
        status: StatusCode::OK,
        headers,
        body: json!({"data": "test"}),
        method: Some(Method::POST),
        uri: Some(Uri::from_static("http://localhost:8888/test")),
    };
    
    let result: Result<Request<Full<Bytes>>, _> = intermediary.try_into();
    
    assert!(result.is_ok());
    let request = result.unwrap();
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/test");
}

#[test]
fn test_intermediary_to_request_missing_method() {
    let intermediary = fips::configuration::intermediary::Intermediary {
        status: StatusCode::OK,
        headers: HeaderMap::new(),
        body: json!({}),
        method: None,
        uri: Some(Uri::from_static("http://localhost:8888/test")),
    };
    
    let result: Result<Request<Full<Bytes>>, _> = intermediary.try_into();
    
    assert!(result.is_err(), "Should fail when method is missing");
}

#[test]
fn test_intermediary_to_request_missing_uri() {
    let intermediary = fips::configuration::intermediary::Intermediary {
        status: StatusCode::OK,
        headers: HeaderMap::new(),
        body: json!({}),
        method: Some(Method::GET),
        uri: None,
    };
    
    let result: Result<Request<Full<Bytes>>, _> = intermediary.try_into();
    
    assert!(result.is_err(), "Should fail when URI is missing");
}

#[test]
fn test_intermediary_preserves_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("x-request-id", "12345".parse().unwrap());
    headers.insert("x-api-key", "secret".parse().unwrap());
    
    let intermediary = fips::configuration::intermediary::Intermediary {
        status: StatusCode::CREATED,
        headers: headers.clone(),
        body: json!({"id": 1}),
        method: Some(Method::POST),
        uri: Some(Uri::from_static("http://localhost:8888/api")),
    };
    
    let response: Response<Full<Bytes>> = intermediary.clone().into();
    
    assert_eq!(
        response.headers().get("x-request-id").unwrap(),
        "12345"
    );
    assert_eq!(
        response.headers().get("x-api-key").unwrap(),
        "secret"
    );
    
    let request: Result<Request<Full<Bytes>>, _> = intermediary.try_into();
    assert!(request.is_ok());
    let request = request.unwrap();
    
    assert_eq!(
        request.headers().get("x-request-id").unwrap(),
        "12345"
    );
}

#[test]
fn test_intermediary_status_codes() {
    let test_statuses = vec![
        StatusCode::OK,
        StatusCode::CREATED,
        StatusCode::NO_CONTENT,
        StatusCode::BAD_REQUEST,
        StatusCode::NOT_FOUND,
        StatusCode::INTERNAL_SERVER_ERROR,
    ];
    
    for status in test_statuses {
        let intermediary = fips::configuration::intermediary::Intermediary {
            status,
            headers: HeaderMap::new(),
            body: json!({}),
            method: None,
            uri: None,
        };
        
        let response: Response<Full<Bytes>> = intermediary.into();
        assert_eq!(response.status(), status);
    }
}
