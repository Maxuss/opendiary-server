#![allow(non_snake_case)]

use crate::{IntoResponse, Uri};

use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;

use serde::Serialize;



pub async fn handler404(path: Uri) -> (StatusCode, Json<Error>) {
    (
        StatusCode::NOT_FOUND,
        Json(Error::NotFound {
            message: format!("Invalid path: {}", path),
        }),
    )
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Maybe<T> {
    Nothing(Error),
    Fine(Success<T>),
}

pub fn Fine<V>(v: V) -> Maybe<V>
where
    V: Serialize,
{
    Maybe::Fine(Success::of(v))
}

pub fn Nothing<V>(err: Error) -> Maybe<V> {
    Maybe::Nothing(err)
}

#[derive(Debug, Clone, Serialize)]
pub struct Success<V> {
    success: bool,
    #[serde(flatten)]
    value: V,
}

impl<T> IntoResponse for Maybe<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match self {
            Maybe::Nothing(err) => Json::into_response(Json(err)),
            Maybe::Fine(success) => Json::into_response(Json(success)),
        }
    }
}

impl<V: Serialize> Success<V> {
    pub fn of(value: V) -> Self {
        Self {
            success: true,
            value,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "error")]
pub enum Error {
    NotFound { message: String },
    InternalError { kind: &'static str, message: String },
    Unknown { message: String },
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        Json::into_response(Json(self))
    }
}

impl Error {
    pub fn unknown<S: Into<String>>(msg: S) -> Error {
        Error::Unknown {
            message: msg.into(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(io: std::io::Error) -> Self {
        Self::InternalError {
            kind: "IOError",
            message: io.to_string(),
        }
    }
}

impl From<uuid::Error> for Error {
    fn from(id: uuid::Error) -> Self {
        Self::InternalError {
            kind: "UUIDError",
            message: id.to_string(),
        }
    }
}

impl From<postcard::Error> for Error {
    fn from(err: postcard::Error) -> Self {
        Self::InternalError {
            kind: "SerializationError",
            message: err.to_string(),
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self::Unknown {
            message: err.to_string(),
        }
    }
}
