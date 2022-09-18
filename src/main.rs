pub mod auth;
pub mod err;
pub mod io;
pub mod models;

use axum::{response::IntoResponse, routing::get, routing::post, Extension, Json, Router};

use crate::err::{Error, Fine, Maybe, Nothing};

use axum::http::Uri;

use serde::Serialize;
use std::net::SocketAddr;

use axum::handler::Handler;

use sqlx::postgres::PgPoolOptions;

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
    let dburl = std::env::var("POSTGRES_DATABASE")
        .expect("`POSTGRES_DATABASE` environment variable not provided!");

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
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
