pub mod err;
pub mod io;

use axum::{response::IntoResponse, routing::get, routing::post, Json, Router};

use crate::err::{Error, Fine, Maybe, Nothing};
use crate::io::{create_io_file, read_io_file};

use axum::extract::Path;
use axum::http::Uri;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use std::str::FromStr;

use tokio::io::{AsyncWriteExt, BufWriter};

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
    let app = Router::new()
        .route("/user/create", post(create_user))
        .route("/user/read/:user", get(read_user))
        .route("/tests/err", get(test_err))
        .fallback(err::handler404);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    log::info!("Starting OpenDiary HTTP Server on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn create_user(Json(body): Json<CreateUser>) -> Payload<UserData> {
    let uid = Uuid::new_v4();
    let data = UserData {
        name: body.name,
        creator: body.creator,
        uuid: uid,
        created_at: Utc::now(),
    };
    let bytes = postcard::to_allocvec(&data)?;
    let file = create_io_file(format!("diary/users/{}.dat", uid)).await?;
    let mut writer = BufWriter::new(file);
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    proceeds(data)
}

async fn read_user(Path(user): Path<String>) -> Payload<UserData> {
    let uid = Uuid::from_str(&user)?;
    let data = read_io_file(format!("diary/users/{}.dat", uid)).await?;
    proceeds(postcard::from_bytes::<UserData>(&data)?)
}

async fn test_err() -> Payload<String> {
    bails("This is an error!")
}

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    creator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserData {
    name: String,
    creator: String,
    uuid: Uuid,
    created_at: DateTime<Utc>,
}
