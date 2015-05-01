use hyper::{Client, HttpError};
use hyper::client::Response;
use hyper::header::ContentType;
use hyper::method::Method;

pub fn put(url: String, body: String) -> Result<Response, HttpError> {
    request_with_body(Method::Put, url, body)
}

pub fn delete(url: String, body: String) -> Result<Response, HttpError> {
    request_with_body(Method::Delete, url, body)
}

fn request_with_body(method: Method, url: String, body: String) -> Result<Response, HttpError> {
    let mut client = Client::new();
    let content_type: ContentType = ContentType(
        "application/x-www-form-urlencoded".parse().unwrap()
    );

    client.request(method, &url).header(content_type).body(&body).send()
}
