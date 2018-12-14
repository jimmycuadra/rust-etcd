//! Contains etcd error types.

use std::convert::From;
use std::error::Error as StdError;
use std::fmt::{Display, Error as FmtError, Formatter};

use http::uri::InvalidUri;
use hyper::{Error as HttpError, StatusCode};
#[cfg(feature = "tls")]
use native_tls::Error as TlsError;
use serde_derive::{Deserialize, Serialize};
use serde_json::Error as SerializationError;
use tokio_timer::timeout::Error as TokioTimeoutError;
use url::ParseError as UrlError;

/// An error returned by an etcd API endpoint.
///
/// This is a logical error, as opposed to other types of errors that may occur when using this
/// crate, such as network or serialization errors. See `Error` for the other types of errors.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ApiError {
    /// The key that was being operated upon or reason for the failure.
    pub cause: Option<String>,
    /// The etcd error code.
    #[serde(rename = "errorCode")]
    pub error_code: u64,
    /// The etcd index.
    pub index: u64,
    /// A human-friendly description of the error.
    pub message: String,
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{}", self.message)
    }
}

impl StdError for ApiError {
    fn description(&self) -> &str {
        &self.message
    }
}

/// An error returned when an operation fails for some reaosn.
#[derive(Debug)]
pub enum Error {
    /// An error returned by an etcd API endpoint.
    Api(ApiError),
    /// An error at the HTTP protocol layer.
    Http(HttpError),
    /// An error returned when invalid conditions have been provided for a compare-and-delete or
    /// compare-and-swap operation.
    InvalidConditions,
    /// An error returned when an etcd cluster member's endpoint is not a valid URI.
    InvalidUri(InvalidUri),
    /// An error returned when the URL for a specific API endpoint cannot be generated.
    InvalidUrl(UrlError),
    /// An error returned when attempting to create a client without at least one member endpoint.
    NoEndpoints,
    /// An error returned when attempting to deserializing invalid JSON.
    Serialization(SerializationError),
    /// An error returned when configuring TLS.
    #[cfg(feature = "tls")]
    Tls(TlsError),
    /// An error returned when an unexpected HTTP status code is returned by the server.
    UnexpectedStatus(StatusCode),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        match *self {
            Error::Api(ref error) => write!(f, "{}", error),
            Error::Http(ref error) => write!(f, "{}", error),
            ref error @ Error::InvalidConditions => write!(f, "{}", error.description()),
            Error::InvalidUri(ref error) => write!(f, "{}", error),
            Error::InvalidUrl(ref error) => write!(f, "{}", error),
            ref error @ Error::NoEndpoints => write!(f, "{}", error.description()),
            #[cfg(feature = "tls")]
            Error::Tls(ref error) => write!(f, "{}", error),
            Error::Serialization(ref error) => write!(f, "{}", error),
            Error::UnexpectedStatus(ref status) => write!(
                f,
                "the etcd server returned an unexpected HTTP status code: {}",
                status
            ),
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Api(_) => "the etcd server returned an error",
            Error::Http(_) => "an error occurred during the HTTP request",
            Error::InvalidConditions => "current value or modified index is required",
            Error::InvalidUri(_) => "a supplied endpoint could not be parsed as a URI",
            Error::InvalidUrl(_) => "a URL for the request could not be generated",
            Error::NoEndpoints => "at least one endpoint is required to create a Client",
            #[cfg(feature = "tls")]
            Error::Tls(_) => "an error occurred configuring TLS",
            Error::Serialization(_) => "an error occurred deserializing JSON",
            Error::UnexpectedStatus(_) => "the etcd server returned an unexpected HTTP status code",
        }
    }
}

impl From<HttpError> for Error {
    fn from(error: HttpError) -> Error {
        Error::Http(error)
    }
}

#[cfg(feature = "tls")]
impl From<TlsError> for Error {
    fn from(error: TlsError) -> Error {
        Error::Tls(error)
    }
}

impl From<UrlError> for Error {
    fn from(error: UrlError) -> Error {
        Error::InvalidUrl(error)
    }
}

impl From<SerializationError> for Error {
    fn from(error: SerializationError) -> Error {
        Error::Serialization(error)
    }
}

impl From<InvalidUri> for Error {
    fn from(error: InvalidUri) -> Error {
        Error::InvalidUri(error)
    }
}

/// An error returned by `kv::watch`.
#[derive(Debug)]
pub enum WatchError {
    /// An error for each failed request to an etcd member.
    Other(Vec<Error>),
    /// The supplied timeout was reached before any request successfully completed.
    Timeout,
}

impl<T> From<TokioTimeoutError<T>> for WatchError {
    fn from(_: TokioTimeoutError<T>) -> Self {
        WatchError::Timeout
    }
}

impl Display for WatchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        match *self {
            WatchError::Timeout => write!(f, "{}", self.description()),
            ref other => other.fmt(f),
        }
    }
}

impl StdError for WatchError {
    fn description(&self) -> &str {
        "operation timed out"
    }
}
