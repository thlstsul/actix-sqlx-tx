use actix_web::{
    http::{header::ContentType, StatusCode},
    HttpResponse, ResponseError,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Indicates that the [`Layer`](crate::Layer) middleware was not installed.
    #[error("required extension not registered; did you add the axum_sqlx_tx::Layer middleware?")]
    MissingExtension,

    /// Indicates that [`Tx`] was extracted multiple times in a single handler/middleware.
    #[error("axum_sqlx_tx::Tx extractor used multiple times in the same handler/middleware")]
    OverlappingExtractors,

    /// A database error occurred when starting the transaction.
    #[error(transparent)]
    Database {
        #[from]
        error: sqlx::Error,
    },
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
