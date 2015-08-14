use hyper::{Client, Error};
use hyper::client::Response;
use hyper::header::ContentType;
use hyper::method::Method;

/// Makes a DELETE request to etcd.
pub fn delete(url: String) -> Result<Response, Error> {
    request(Method::Delete, url)
}

/// Makes a GET request to etcd.
pub fn get(url: String) -> Result<Response, Error> {
    request(Method::Get, url)
}

/// Makes a POST request to etcd.
pub fn post(url: String, body: String) -> Result<Response, Error> {
    request_with_body(Method::Post, url, body)
}

/// Makes a PUT request to etcd.
pub fn put(url: String, body: String) -> Result<Response, Error> {
    request_with_body(Method::Put, url, body)
}

// private

/// Makes a request to etcd.
fn request(method: Method, url: String) -> Result<Response, Error> {
    let client = Client::new();
    let request = client.request(method, &url);

    request.send()
}

/// Makes a request with an HTTP body to etcd.
fn request_with_body(method: Method, url: String, body: String) -> Result<Response, Error> {
    let client = Client::new();
    let content_type: ContentType = ContentType(
        "application/x-www-form-urlencoded".parse().unwrap()
    );
    let request = client.request(method, &url).header(content_type).body(&body);

    request.send()
}
