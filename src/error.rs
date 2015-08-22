//! Contains etcd error types.

use std::convert::From;
use hyper::Error as HttpError;
use std::io::Error as IoError;

/// An error returned by `Client` when an API call fails.
#[derive(Debug)]
pub enum Error {
    /// An error returned by etcd.
    Etcd(EtcdError),
    /// An HTTP error from attempting to connect to etcd.
    Http(HttpError),
    /// An IO error, which can happen when reading the HTTP response.
    Io(IoError),
    /// An error returned when invalid conditions have been provided for a compare-and-delete or
    /// compare-and-swap operation.
    InvalidConditions(&'static str),
}

/// An error returned by etcd.
#[derive(Debug, RustcDecodable)]
#[allow(non_snake_case)]
pub struct EtcdError {
    /// The key that was being operated upon or reason for the failure.
    pub cause: Option<String>,
    /// The etcd error code.
    pub errorCode: u64,
    /// The etcd index.
    pub index: u64,
    /// A human-friendly description of the error.
    pub message: String,
}

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
