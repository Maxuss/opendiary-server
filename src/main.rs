use axum::{
    routing::{get, post},
    http::StatusCode,
    response::IntoResponse,
    Json, Router};

use std::net::SocketAddr;
use axum::extract::Path;
use axum::http::Uri;
use serde::{Deserialize, Serialize};

pub type RefStr = &'static str;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let app = Router::new()
        .route("/", get(root_noarg))
        .route("/hello/:name", get(root_args))
        .fallback(fallback);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    log::info!("Starting OpenDiary HTTP Server on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn root_noarg() -> RefStr {
    "No arguments provided!"
}

async fn root_args(Path(name): Path<String>) -> String {
    format!("Test: {}", name)
}

async fn fallback(uri: Uri) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("404 `{}` Path Not Found", uri))
}