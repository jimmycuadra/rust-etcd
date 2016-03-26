use std::convert::From;
use std::io::Error as IoError;

use hyper::Error as HttpError;
use url::ParseError;

/// An error returned by `Client` method failures.
#[derive(Debug)]
pub enum Error {
    /// An error returned by etcd.
    Api(ApiError),
    /// An HTTP error from attempting to connect to etcd.
    Http(HttpError),
    /// An IO error, which can happen when reading the HTTP response.
    Io(IoError),
    /// An error returned when invalid conditions have been provided for a compare-and-delete or
    /// compare-and-swap operation.
    InvalidConditions(&'static str),
    /// An error if an etcd cluster member's endpoint is not a valid URL.
    InvalidUrl(ParseError),
    /// An error when attempting to create a client without at least one member endpoint.
    NoEndpoints,
}

/// An error returned by etcd.
#[derive(Debug, Deserialize)]
pub struct ApiError {
    /// The key that was being operated upon or reason for the failure.
    pub cause: Option<String>,
    /// The etcd error code.
    #[serde(rename="errorCode")]
    pub error_code: u64,
    /// The etcd index.
    pub index: u64,
    /// A human-friendly description of the error.
    pub message: String,
}

/// The result type returned by `Client` methods.
pub type EtcdResult<T> = Result<T, Error>;

impl From<HttpError> for Error {
    fn from(error: HttpError) -> Error {
        Error::Http(error)
    }
}

impl From<IoError> for Error {
    fn from(error: IoError) -> Error {
        Error::Io(error)
    }
}

impl From<ParseError> for Error {
    fn from(error: ParseError) -> Error {
        Error::InvalidUrl(error)
    }
}
