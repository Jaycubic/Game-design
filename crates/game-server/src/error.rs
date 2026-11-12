//! API errors with stable codes and safe messages. Internal details are logged,
//! never returned to clients.

use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{message}")]
    Client { status: u16, code: String, message: String },
    #[error("database unavailable")]
    Database(#[from] sqlx::Error),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

impl ApiError {
    pub fn new(status: u16, code: &str, message: impl Into<String>) -> Self {
        ApiError::Client { status, code: code.into(), message: message.into() }
    }
    pub fn bad_request(code: &str, message: impl Into<String>) -> Self {
        Self::new(400, code, message)
    }
    pub fn unauthorized() -> Self {
        Self::new(401, "unauthorized", "Please sign in again.")
    }
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(403, "forbidden", message)
    }
    pub fn not_found(what: &str) -> Self {
        Self::new(404, "not_found", format!("{what} was not found."))
    }
    pub fn conflict(code: &str, message: impl Into<String>) -> Self {
        Self::new(409, code, message)
    }
    pub fn too_many() -> Self {
        Self::new(429, "rate_limited", "Too many attempts — wait a minute and try again.")
    }

    pub fn body(&self) -> ErrorBody {
        match self {
            ApiError::Client { code, message, .. } => ErrorBody { code: code.clone(), message: message.clone() },
            ApiError::Database(_) => ErrorBody {
                code: "database_unavailable".into(),
                message: "The game database is unavailable. Check that PostgreSQL is running, then try again.".into(),
            },
            ApiError::Internal(_) => {
                ErrorBody { code: "internal".into(), message: "Something went wrong on the server.".into() }
            }
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::Client { status, .. } => StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_REQUEST),
            ApiError::Database(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::Database(e) => tracing::error!(error = %e, "database error"),
            ApiError::Internal(e) => tracing::error!(error = %e, "internal error"),
            ApiError::Client { .. } => {}
        }
        HttpResponse::build(self.status_code()).json(self.body())
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
