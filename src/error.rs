use std::convert::From;

use hyper::HttpError;

#[derive(Debug)]
pub enum Error {
    Etcd(EtcdError),
    Http(HttpError),
}

#[derive(Debug, RustcDecodable)]
#[allow(non_snake_case)]
pub struct EtcdError {
    pub cause: Option<String>,
    pub errorCode: u64,
    pub index: u64,
    pub message: String,
}

impl From<HttpError> for Error {
    fn from(error: HttpError) -> Error {
        Error::Http(error)
    }
}
