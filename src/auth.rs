use axum::extract::Path;
use axum::{Extension, Json};
use chrono::{DateTime, Duration, Utc};
use pbkdf2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use pbkdf2::Pbkdf2;
use rand::{thread_rng, Rng};
use rand_core::OsRng;
use serde::{Deserialize, Serialize, Serializer};
use serde_with::skip_serializing_none;
use sha2::{Digest, Sha256};
use std::ops::Add;

use crate::models::{StudentData, StudentSession};
use crate::{breaks, proceeds, Error, Payload};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Eq, Ord, PartialOrd, PartialEq)]
pub enum AuthResult {
    Success,
    SessionExpired,
    InvalidSession,
}

impl Serialize for AuthResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:?}", self))
    }
}

pub async fn drop_session(
    Json(EnsureSession { ssid, value }): Json<EnsureSession<DropSession>>,
    Extension(pg): Extension<PgPool>,
) -> Payload<SessionBasedResponse<SessionDropped>> {
    let auth_result = ensure_authenticated(Some(ssid.clone()), &pg).await?;
    if auth_result != AuthResult::Success {
        return proceeds(SessionBasedResponse {
            auth_result,
            value: None,
        });
    }

    let affected = sqlx::query("DELETE FROM user_sessions WHERE ssid = $1 AND belongs_to = $2")
        .bind(&ssid)
        .bind(&value.uuid)
        .execute(&pg)
        .await
        .map_err(Error::from)?;

    return proceeds(SessionBasedResponse {
        auth_result,
        value: Some(SessionDropped {
            student_id: value.uuid,
            drop_success: affected.rows_affected() >= 1,
        }),
    });
}

pub async fn ensure_authenticated(
    session_id: Option<String>,
    pg: &PgPool,
) -> anyhow::Result<AuthResult, Error> {
    return if let None = session_id {
        Ok(AuthResult::InvalidSession)
    } else if let Some(ssid) = session_id {
        if ssid.is_empty() {
            return Ok(AuthResult::InvalidSession);
        }
        let session = sqlx::query_as::<_, StudentSession>(
            "SELECT * FROM user_sessions WHERE ssid = $1 LIMIT 1",
        )
        .bind(&ssid)
        .fetch_optional(pg)
        .await
        .map_err(Error::from)?;

        if let Some(session) = session {
            let expires_at = session.expires_at;
            if Utc::now().gt(&expires_at) {
                sqlx::query("DELETE FROM user_sessions WHERE ssid = $1")
                    .bind(&ssid)
                    .execute(pg)
                    .await
                    .map_err(Error::from)?;
                return Ok(AuthResult::InvalidSession);
            }
            Ok(AuthResult::Success)
        } else {
            Ok(AuthResult::InvalidSession)
        }
    } else {
        Ok(AuthResult::InvalidSession)
    };
}

pub async fn login_student(
    Json(login): Json<LoginStudent>,
    Extension(pg): Extension<PgPool>,
) -> Payload<LoggedInStudent> {
    if login.password.is_empty() {
        return breaks(Error::InvalidPayload {
            message: "`password` parameter was empty".to_string(),
        });
    }

    let user = sqlx::query_as::<_, StudentData>("SELECT * FROM users WHERE uuid = $1 LIMIT 1")
        .bind(&login.uuid)
        .fetch_optional(&pg)
        .await
        .map_err(Error::from)?;

    let student = if let Some(user) = user {
        user
    } else {
        return breaks(Error::UserDoesNotExist {
            message: format!("User with uuid `{}` does not exist!", login.uuid),
        });
    };
    let hash = PasswordHash::new(&student.password_hash).map_err(Error::from)?;
    let matches = Pbkdf2
        .verify_password(login.password.as_bytes(), &hash)
        .is_ok();
    if !matches {
        return breaks(Error::AuthenticationFailure {
            message: "Passwords do not match!".to_string(),
        });
    }

    let existing_session = sqlx::query_as::<_, StudentSession>(
        "SELECT * FROM user_sessions WHERE belongs_to = $1 LIMIT 1",
    )
    .bind(login.uuid)
    .fetch_optional(&pg)
    .await
    .map_err(Error::from)?;

    if let Some(existing) = existing_session {
        // already authenticated
        return proceeds(LoggedInStudent {
            session_id: existing.ssid,
            student_id: existing.belongs_to,
            expires_at: existing.expires_at,
        });
    }

    let ssid_bytes: [u8; 32] = thread_rng().gen();

    let mut hasher: Sha256 = Digest::new();
    hasher.update(&ssid_bytes);
    let result = hasher.finalize();
    let ssid = hex::encode(result);

    let expires_in = Duration::days(2);
    let expires_at = Utc::now().add(expires_in);
    let res = sqlx::query("INSERT INTO user_sessions VALUES($1, $2, $3)")
        .bind(&ssid)
        .bind(&expires_at)
        .bind(&student.uuid)
        .execute(&pg)
        .await
        .map_err(Error::from)?;

    if res.rows_affected() < 1 {
        return breaks(Error::InternalError {
            kind: "DatabaseError",
            message: "Could not update session ids!".to_string(),
        });
    }

    return proceeds(LoggedInStudent {
        session_id: ssid,
        student_id: student.uuid,
        expires_at,
    });
}

pub async fn query_user_id(
    Path(username): Path<String>,
    Extension(pg): Extension<PgPool>,
) -> Payload<CreatedStudent> {
    if username.is_empty() {
        return breaks(Error::InvalidPayload {
            message: "`username` parameter was empty".to_string(),
        });
    }

    let user = sqlx::query_as::<_, StudentData>("SELECT * FROM users WHERE username = $1 LIMIT 1")
        .bind(&username)
        .fetch_optional(&pg)
        .await
        .map_err(Error::from)?;

    return if let Some(user) = user {
        proceeds(CreatedStudent {
            student_id: user.uuid,
        })
    } else {
        breaks(Error::UserDoesNotExist {
            message: format!("User with name `{}` does not exist!", username),
        })
    };
}

pub async fn register_student(
    Json(student): Json<CreateStudent>,
    Extension(pg): Extension<PgPool>,
) -> Payload<CreatedStudent> {
    if student.password.is_empty() {
        return breaks(Error::MissingCredentials {
            message: "Provided password was empty!".to_string(),
        });
    }

    let user = sqlx::query_as::<_, StudentData>(
        "SELECT * FROM users WHERE username = $2 OR email = $1 LIMIT 1",
    )
    .bind(&student.email)
    .bind(&student.username)
    .fetch_optional(&pg)
    .await
    .map_err(Error::from)?;
    if let Some(_) = user {
        return breaks(Error::UserAlreadyExists {
            message: "User with provided email/username already exists!".to_string(),
        });
    }

    let user = StudentData {
        uuid: Uuid::new_v4(),
        username: student.username,
        name: student.name,
        surname: student.surname,
        patronymic: student.patronymic,
        email: student.email,
        password_hash: Pbkdf2
            .hash_password(
                student.password.as_bytes(),
                &SaltString::generate(&mut OsRng),
            )?
            .to_string(),
        created_at: Utc::now(),
    };

    let res = sqlx::query("INSERT INTO users VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
        .bind(user.uuid.clone())
        .bind(user.username)
        .bind(user.name)
        .bind(user.surname)
        .bind(user.patronymic)
        .bind(user.email)
        .bind(user.password_hash)
        .bind(user.created_at)
        .execute(&pg)
        .await
        .map_err(|err| Error::InternalError {
            kind: "DatabaseError",
            message: format!("{:?}", err),
        })?;

    if res.rows_affected() < 1 {
        return breaks(Error::InternalError {
            kind: "DatabaseError",
            message: "Could not save data to database!".to_string(),
        });
    } else {
        proceeds(CreatedStudent {
            student_id: user.uuid,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionDropped {
    pub student_id: Uuid,
    pub drop_success: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DropSession {
    pub uuid: Uuid,
}

#[derive(Debug, Clone, Serialize)]
#[skip_serializing_none]
pub struct SessionBasedResponse<V> {
    pub auth_result: AuthResult,
    #[serde(flatten)]
    pub value: Option<V>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnsureSession<V> {
    pub ssid: String,
    #[serde(flatten)]
    pub value: V,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionAlreadyExists {
    status: String,
    #[serde(flatten)]
    session: LoggedInStudent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggedInStudent {
    session_id: String,
    student_id: Uuid,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatedStudent {
    student_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginStudent {
    uuid: Uuid,
    password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateStudent {
    pub username: String,
    pub name: String,
    pub surname: String,
    pub patronymic: Option<String>,
    pub email: String,
    pub password: String,
}
