//! Contains etcd error types.

use std::convert::From;
use std::error::Error as StdError;
use std::fmt::{Display, Error as FmtError, Formatter};
use std::io::Error as IoError;

use hyper::Error as HttpError;
use hyper::error::UriError;
#[cfg(feature = "tls")]
use native_tls::Error as NativeTlsError;
use serde_json::Error as SerdeJsonError;
use url::ParseError;

/// An error returned by `Client` method failures.
#[derive(Debug)]
pub enum Error {
    /// An error returned by etcd.
    Api(ApiError),
    /// An HTTP error from attempting to connect to etcd.
    Http(HttpError),
    /// An error returned when invalid conditions have been provided for a compare-and-delete or
    /// compare-and-swap operation.
    InvalidConditions(&'static str),
    /// An error if an etcd cluster member's endpoint is not a valid URI.
    InvalidUri(UriError),
    /// An error if the URL for a request cannot be generated.
    InvalidUrl(ParseError),
    /// An IO error, which can happen when reading the HTTP response.
    Io(IoError),
    /// An error configuring TLS.
    #[cfg(feature = "tls")]
    NativeTls(NativeTlsError),
    /// An error deserializing JSON.
    Serialization(SerdeJsonError),
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

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        match *self {
            Error::Api(ref error) => write!(f, "{}", error),
            Error::Http(ref error) => write!(f, "{}", error),
            Error::InvalidConditions(reason) => write!(f, "{}", reason),
            Error::InvalidUri(ref error) => write!(f, "{}", error),
            Error::InvalidUrl(ref error) => write!(f, "{}", error),
            Error::Io(ref error) => write!(f, "{}", error),
            #[cfg(feature = "tls")]
            Error::NativeTls(ref error) => write!(f, "{}", error),
            Error::Serialization(ref error) => write!(f, "{}", error),
            Error::NoEndpoints => f.write_str("At least one endpoint is required to create a Client"),
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Api(_) => "the etcd server returned an error",
            Error::Http(_) => "an error occurred during the HTTP request",
            Error::InvalidConditions(conditions) => conditions,
            Error::InvalidUri(_) => "a supplied endpoint could not be parsed as a URI" ,
            Error::InvalidUrl(_) => "a URL for the request could not be generated",
            Error::Io(_) => "an error occurred trying to read etcd's HTTP response",
            #[cfg(feature = "tls")]
            Error::NativeTls(_) => "an error occurred configuring TLS",
            Error::Serialization(_) => "an error occurred deserializing JSON",
            Error::NoEndpoints => "at least one endpoint is required to create a Client",
        }
    }
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

#[cfg(feature = "tls")]
impl From<NativeTlsError> for Error {
    fn from(error: NativeTlsError) -> Error {
        Error::NativeTls(error)
    }
}

impl From<ParseError> for Error {
    fn from(error: ParseError) -> Error {
        Error::InvalidUrl(error)
    }
}

impl From<SerdeJsonError> for Error {
    fn from(error: SerdeJsonError) -> Error {
        Error::Serialization(error)
    }
}

impl From<UriError> for Error {
    fn from(error: UriError) -> Error {
        Error::InvalidUri(error)
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "{}", self.message)
    }
}

impl StdError for ApiError {
    fn description(&self) -> &str {
        &self.message
    }
}
