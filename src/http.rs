use std::io::Read;

use hyper::{Client, HttpError};
use hyper::client::Response;
use hyper::header::ContentType;
use hyper::method::Method;
use hyper::status::StatusCode;

use rustc_serialize::{json, Decodable};

use error::Error;

/// Makes a DELETE request to etcd.
pub fn delete(url: String) -> Result<Response, HttpError> {
    request(Method::Delete, url)
}

/// Makes a GET request to etcd.
pub fn get(url: String) -> Result<Response, HttpError> {
    request(Method::Get, url)
}

/// Makes a PUT request to etcd.
pub fn put(url: String, body: String) -> Result<Response, HttpError> {
    request_with_body(Method::Put, url, body)
}

/// Read response and decode JSON
pub fn decode<T: Decodable>(mut response: Response) -> Result<T, Error> {
    let mut response_body = String::new();
    try!(response.read_to_string(&mut response_body));

    match response.status {
        StatusCode::Created | StatusCode::Ok => Ok(json::decode(&response_body).unwrap()),
        _ => Err(Error::Etcd(json::decode(&response_body).unwrap()))
    }
}


// private

/// Makes a request to etcd.
fn request(method: Method, url: String) -> Result<Response, HttpError> {
    let mut client = Client::new();

    client.request(method, &url).send()
}

/// Makes a request with an HTTP body to etcd.
fn request_with_body(method: Method, url: String, body: String) -> Result<Response, HttpError> {
    let mut client = Client::new();
    let content_type: ContentType = ContentType(
        "application/x-www-form-urlencoded".parse().unwrap()
    );

    client.request(method, &url).header(content_type).body(&body).send()
}
