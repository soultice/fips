use crate::{
    configuration::nconfiguration::{
        AsyncFrom, Intermediary, NConfiguration,
        RuleAndIntermediaryHolder, RuleSet,
    },
    utility::log::{Loggable, LoggableType, RequestInfo, ResponseInfo},
    PaintLogsCallbacks,
};

use hyper::{
    header::{HeaderMap, HeaderValue},
    Body, Client, Method, Request, Response, StatusCode,
};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

// this should be segmented with better care, split into smaller functions, move everything possible from state to separate arguments
pub async fn routes(
    req: Request<Body>,
    configuration: Arc<AsyncMutex<NConfiguration>>,
    logging: &Arc<PaintLogsCallbacks>,
) -> Result<Response<Body>, hyper::Error> {
    let requestinfo = RequestInfo::from(&req);
    let log_output = Loggable {
        message_type: LoggableType::IncomingRequestAtFfips(requestinfo),
        message: "".to_owned(),
    };
    (logging.0)(&log_output);

    let intermediary = Intermediary::async_from(req).await;

    let c = intermediary.clone();
    //TODO clean up adding cors, have rule that makes sense here
    if let (Some(method), Some(uri)) = (&c.method, &c.uri) {
        if method == Method::OPTIONS {
            let mut resp = Response::new(Body::from(""));
            add_cors_headers(resp.headers_mut());
            return Ok(resp);
        }
        if method == Method::OPTIONS && uri == "/favicon.ico" {
            //early return for favicon
            return Ok(Response::new(Body::default()));
        }
    }
    // find first matching rule
    let config = configuration.lock().await;
    let matching_rule_idx = config.rules.iter().enumerate().find_map(|(idx, rule)| match rule {
        RuleSet::Rule(rule) => {
            if rule.should_apply(&intermediary) {
                Some(idx)
            } else {
                None
            }
        }
    });
    drop(config);


    if let Some(idx) = matching_rule_idx {
        //add uri and route from configuration (enrich)
        log::info!("idx: {:?}", idx);
        let config_guard = configuration.lock().await;
        log::info!("guard {:?}", config_guard);
        let rule = config_guard.rules[idx].into_inner();
        log::info!("matching_rule: {:?}", rule);
        let mut holder = RuleAndIntermediaryHolder {
            rule,
            intermediary,
        };

        log::info!("holder");
        let request = hyper::Request::try_from(&holder);
        log::info!("request: {:?}", request);

        // Rule is forwarding (Proxy/FIPS)
        let resp = if let Ok(request) = request {
            let client = Arc::new(Client::new());
            let resp = client.request(request).await?;

            let responseinfo = ResponseInfo::from(&resp);
            let log_output = Loggable {
                message_type: LoggableType::OutGoingResponseFromFips(
                    responseinfo,
                ),
                message: "".to_owned(),
            };
            (logging.0)(&log_output);
            let inter = Intermediary::async_from(resp).await;
            holder.intermediary = inter;
            let resp = Response::async_from(holder).await;
            log::info!("resp: {:?}", resp);
            Ok(resp)
        } else {
            // rule isnt forwarding
            let resp = Response::async_from(holder).await;
            log::info!("resp from non-forwading: {:?}", resp);
            Ok(resp)
        };

        if let Some(with) = &rule.with {
            if let Some(sleep_time_ms) = with.sleep {
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    sleep_time_ms,
                ))
                .await;
            }
        }
        resp

    } else {
        //TODO create this from intermediary
        let mut no_matching_rule =
            Response::new(Body::from("no matching rule found"));
        *no_matching_rule.status_mut() = StatusCode::NOT_FOUND;

        add_cors_headers(no_matching_rule.headers_mut());
        (logging.0)(&Loggable {
            message: format!(
                "No matching rule found for URI: {:?}",
                &intermediary.clone().uri
            ),
            message_type: LoggableType::Plain,
        });
        Ok(no_matching_rule)
    }
}

fn add_cors_headers(headers: &mut HeaderMap) {
    headers
        .insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    headers.insert(
        "Access-Control-Allow-Headers",
        HeaderValue::from_static("*"),
    );
    headers.insert(
        "Access-Control-Allow-Methods",
        HeaderValue::from_static("*"),
    );
}
