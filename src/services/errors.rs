use thiserror::Error;

/// Generic error type used by service layer functions.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ServiceError {
    /// The user is not authorized to perform the operation.
    #[error("unauthorized")]
    Unauthorized,
    /// Requested resource was not found.
    #[error("not found")]
    NotFound,
    /// An unexpected internal error occurred.
    #[error("internal error")]
    Internal,
}

/// Convenient alias for results returned from service functions.
pub type ServiceResult<T> = Result<T, ServiceError>;
