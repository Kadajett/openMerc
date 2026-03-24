// src/webhook.rs
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use std::net::SocketAddr;
use serde_json::Value;
use crate::ci::run_ci_review;

async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    if req.method() != Method::POST {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::empty())
            .unwrap());
    }
    let whole = hyper::body::to_bytes(req.into_body()).await.unwrap();
    let payload: Value = serde_json::from_slice(&whole).unwrap_or_default();
    // Very simple check for PR number in payload
    if let Some(pr) = payload.get("pull_request").and_then(|pr| pr.get("number")).and_then(|n| n.as_u64()) {
        // Trigger CI review (fire and forget)
        tokio::spawn(async move {
            let _ = run_ci_review(pr as u32);
        });
        Ok(Response::new(Body::from("CI triggered")))
    } else {
        Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("No PR number"))
            .unwrap())
    }
}

pub async fn run_server(addr: SocketAddr) {
    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });
    let server = Server::bind(&addr).serve(make_svc);
    println!("Listening on http://{}", addr);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
