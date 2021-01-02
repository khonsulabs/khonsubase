use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct Account {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
}

pub struct AccountEmail {
    pub account_id: i64,
    pub email: Option<String>,
    pub hashed_email: String,
    pub is_primary: bool,
}

pub struct AccountAgreement {
    pub account_id: i64,
    pub agreement: String,
    pub version_agreed_to: String,
    pub agreed_at: DateTime<Utc>,
}

pub struct Installation {
    pub id: Uuid,
    pub account_id: i64,
    pub created_at: DateTime<Utc>,
    pub last_connected_at: DateTime<Utc>,
}
