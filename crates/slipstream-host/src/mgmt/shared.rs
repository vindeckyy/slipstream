//! Shared control-plane plumbing for the management submodules: the [`ApiError`] envelope
//! every non-2xx response wears, [`api_error`], the [`ApiJson`] extractor that keeps axum's own
//! rejections in that envelope, and a small re-export prelude of the axum/serde/utoipa vocabulary
//! the handler modules share. Split out of the `mgmt` facade (plan §W5).

use axum::extract::Request;

// Re-export prelude: the vocabulary every handler submodule pulls in via `use super::shared::*`.
pub(crate) use super::MgmtState;
pub(crate) use axum::extract::{Path, Query, State};
pub(crate) use axum::http::StatusCode;
pub(crate) use axum::response::{IntoResponse, Response};
pub(crate) use axum::Json;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::sync::Arc;
pub(crate) use utoipa::ToSchema;

/// One field-level validation problem, keyed by the dotted setting path
/// (e.g. `audio_video.max_fps`) so the console can anchor the error to the control.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct ApiFieldError {
    pub field: String,
    pub message: String,
}

/// Error envelope for every non-2xx response.
#[derive(Serialize, Deserialize, ToSchema)]
pub(crate) struct ApiError {
    error: String,
    /// Field-keyed validation issues, present only on 400 validation failures.
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<Vec<ApiFieldError>>,
}

pub(crate) fn api_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(ApiError {
            error: message.to_string(),
            fields: None,
        }),
    )
        .into_response()
}

/// 400 with a summary plus field-keyed issues the console can render at each control.
pub(crate) fn api_validation_error(summary: &str, fields: Vec<ApiFieldError>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError {
            error: summary.to_string(),
            fields: Some(fields),
        }),
    )
        .into_response()
}

/// `axum::Json` whose rejections (bad JSON → 400/422, wrong content-type → 415) are
/// rewrapped in the [`ApiError`] envelope, keeping "every non-2xx body is `ApiError`" true.
pub(crate) struct ApiJson<T>(pub(crate) T);

impl<S, T> axum::extract::FromRequest<S> for ApiJson<T>
where
    Json<T>: axum::extract::FromRequest<S, Rejection = axum::extract::rejection::JsonRejection>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(ApiJson(value)),
            Err(rejection) => Err(api_error(rejection.status(), &rejection.body_text())),
        }
    }
}
