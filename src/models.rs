use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StudentData {
    pub uuid: Uuid,
    pub username: String,
    pub name: String,
    pub surname: String,
    pub patronymic: Option<String>,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StudentSession {
    pub ssid: String,
    pub belongs_to: Uuid,
    pub expires_at: DateTime<Utc>,
}
