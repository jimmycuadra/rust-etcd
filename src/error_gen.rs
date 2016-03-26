use std::convert::From;
use hyper::Error as HttpError;
use std::io::Error as IoError;

/// An error returned by `Client` when an API call fails.
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
