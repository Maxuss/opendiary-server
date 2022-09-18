pub mod err;
pub mod io;
pub mod models;
pub mod auth;

use std::mem::MaybeUninit;
use axum::{response::IntoResponse, routing::get, routing::post, Json, Router, Extension};

use crate::err::{Error, Fine, handle_json_error, Maybe, Nothing};
use crate::io::{create_io_file, read_io_file};

use axum::extract::Path;
use axum::http::{Request, StatusCode, Uri};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use anyhow::bail;
use axum::error_handling::{HandleError, HandleErrorLayer};
use axum::extract::rejection::JsonRejection;
use axum::handler::Handler;
use axum::headers::HeaderName;
use axum::middleware::Next;
use axum::response::Response;
use lazy_static::lazy_static;
use sqlx::postgres::PgPoolOptions;
use sync_wrapper::SyncWrapper;

use tokio::io::{AsyncWriteExt, BufWriter};
use tower::ServiceBuilder;

use uuid::Uuid;

pub type RefStr = &'static str;
pub type Payload<T> = axum::response::Result<Json<Maybe<T>>, Error>;

pub fn proceeds<V>(value: V) -> Payload<V>
where
    V: Serialize,
{
    Ok(Json(Fine(value)))
}

pub fn breaks<V>(err: Error) -> Payload<V>
where
    V: Serialize,
{
    Ok(Json(Nothing(err)))
}

pub fn bails<V, S: Into<String>>(err: S) -> Payload<V>
where
    V: Serialize,
{
    Ok(Json(Nothing(Error::InternalError {
        kind: "Unknown",
        message: err.into(),
    })))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    io::prepare_io().await;
    let dburl = std::env::var("POSTGRES_DATABASE").expect("`POSTGRES_DATABASE` environment variable not provided!");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&dburl)
        .await?;

    let app = Router::new()
        .route("/student/register", post(auth::register_student))
        .route("/student/get_id/:username", get(auth::query_user_id))
        .route("/student/login", post(auth::login_student))
        .fallback(err::handler404.into_service())
        .layer(Extension(pool));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    log::info!("Starting OpenDiary HTTP Server on http://{}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service()).await?;

    Ok(())
}