use crate::{
    configuration::nconfiguration::{AsyncFrom, Intermediary, NConfiguration, Rule, RuleSet, RuleAndIntermediaryHolder},
    plugin_registry::ExternalFunctions,
    utility::log::{Loggable, LoggableType, RequestInfo, ResponseInfo},
    PaintLogsCallbacks,
};

use hyper::{
    header::{HeaderMap, HeaderValue},
    Body, Client, Method, Request, Response, StatusCode,
};
use std::sync::{Arc, Mutex};

// this should be segmented with better care, split into smaller functions, move everything possible from state to separate arguments
pub async fn routes(
    req: Request<Body>,
    configuration: Arc<Mutex<NConfiguration>>,
    logging: &Arc<PaintLogsCallbacks>,
) -> Result<Response<Body>, hyper::Error> {
    let requestinfo = RequestInfo::from(&req);
    let log_output = Loggable {
        message_type: LoggableType::IncomingRequestAtFfips(requestinfo),
        message: "".to_owned(),
    };
    (logging.0)(&log_output);

    let intermediary = Intermediary::async_from(req).await;

    let c  = intermediary.clone();
    match (c.method, c.uri) {
        (Some(m), Some(u)) => {
            if u == "/favicon.ico" {
                //early return for favicon
                return Ok(Response::new(Body::default()));
            }
            if m == Method::OPTIONS {
                let mut preflight = Response::new(Body::default());
                add_cors_headers(preflight.headers_mut());
                //early return for preflight
                return Ok(preflight);
            }
        }
        _ => {}
    }

    // find first matching rule
    let config = configuration.lock().unwrap().clone();
    let matching_rule = config.rules.iter().find_map(|rule| match rule {
        RuleSet::Rule(rule) => {
            if rule.should_apply(&intermediary) {
                Some(rule)
            } else {
                None
            }
        }
        _ => None,
    });

    if let Some(r) = matching_rule {
        //add uri and route from configuration (enrich)
        let holder = RuleAndIntermediaryHolder { rule: &r, intermediary: &intermediary };

        let request = hyper::Request::try_from(intermediary.clone());

        // Rule is forwarding (Proxy/FIPS)
        let resp = if let Ok(request) = request {

            let client = Arc::new(Client::new());
            // let resp = client.request(request).await?;

/*             let responseinfo = ResponseInfo::from(&resp.unwrap());
            let log_output = Loggable {
                message_type: LoggableType::OutGoingResponseFromFips(responseinfo),
                message: "".to_owned(),
            };
            (logging.0)(&log_output); */
            //resp
            let resp = Response::from(holder);
            Ok(resp)
        } else {
            // rule isnt forwarding
            let resp = Response::from(holder);
            Ok(resp)
        };

        if let Some(sleep_time_ms) = r.with.sleep {
            tokio::time::sleep(tokio::time::Duration::from_millis(sleep_time_ms)).await;
        }

        return resp;
    } else {
        //TODO create this from intermediary
        let mut no_matching_rule = Response::new(Body::from("no matching rule found"));
        *no_matching_rule.status_mut() = StatusCode::NOT_FOUND;

        add_cors_headers(no_matching_rule.headers_mut());
        (logging.0)(&Loggable {
            message: format!(
                "No matching rule found for URI: {:?}",
                &intermediary.clone().uri
            ),
            message_type: LoggableType::Plain,
        });
        return Ok(no_matching_rule);
    }
}

fn add_cors_headers(headers: &mut HeaderMap) {
    headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    headers.insert(
        "Access-Control-Allow-Headers",
        HeaderValue::from_static("*"),
    );
    headers.insert(
        "Access-Control-Allow-Methods",
        HeaderValue::from_static("*"),
    );
}
