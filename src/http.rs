use hyper::{Client, HttpError};
use hyper::client::Response;
use hyper::header::ContentType;
use hyper::method::Method;

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
