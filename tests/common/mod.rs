/// Common test utilities and helpers

use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode, Uri};
use http_body_util::Full;
use hyper::{Request, Response};
use serde_json::json;

/// Create a test intermediary with common defaults
pub fn create_test_intermediary() -> fips::configuration::intermediary::Intermediary {
    fips::configuration::intermediary::Intermediary {
        status: StatusCode::OK,
        headers: HeaderMap::new(),
        body: json!({"test": "data"}),
        method: Some(Method::GET),
        uri: Some(Uri::from_static("http://localhost:8888/test")),
    }
}

/// Create a test HTTP request
pub fn create_test_request(uri: &str, method: Method, body: &str) -> Request<Full<Bytes>> {
    Request::builder()
        .uri(uri)
        .method(method)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

/// Create a test HTTP response
pub fn create_test_response(status: StatusCode, body: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

/// Load test configuration from nconfig-test directory
pub fn get_test_config_path() -> std::path::PathBuf {
    std::path::PathBuf::from("./nconfig-test")
}
