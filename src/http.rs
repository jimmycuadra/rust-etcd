use hyper::{Client, Error};
use hyper::client::Response;
use hyper::header::ContentType;
use hyper::method::Method;
use hyper::net::{HttpsConnector, Openssl};

#[derive(Debug)]
pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    /// Create a new `HttpClient`.
    pub fn new() -> Self {
        HttpClient {
            client: Client::new(),
        }
    }

    pub fn https(cert: &str, key: &str) -> Result<Self, Error> {
        let openssl = try!(Openssl::with_cert_and_key(cert, key));
        let connector = HttpsConnector::new(openssl);

        Ok(HttpClient {
            client: Client::with_connector(connector),
        })
    }

    /// Makes a DELETE request to etcd.
    pub fn delete(&self, url: String) -> Result<Response, Error> {
        self.request(Method::Delete, url)
    }

    /// Makes a GET request to etcd.
    pub fn get(&self, url: String) -> Result<Response, Error> {
        self.request(Method::Get, url)
    }

    /// Makes a POST request to etcd.
    pub fn post(&self, url: String, body: String) -> Result<Response, Error> {
        self.request_with_body(Method::Post, url, body)
    }

    /// Makes a PUT request to etcd.
    pub fn put(&self, url: String, body: String) -> Result<Response, Error> {
        self.request_with_body(Method::Put, url, body)
    }

    // private

    /// Makes a request to etcd.
    fn request(&self, method: Method, url: String) -> Result<Response, Error> {
        self.client.request(method, &url).send()
    }

    /// Makes a request with an HTTP body to etcd.
    fn request_with_body(&self,
        method: Method,
        url: String,
        body: String,
    ) -> Result<Response, Error> {
        let content_type = ContentType::form_url_encoded();

        self.client.request(method, &url).header(content_type).body(&body).send()
    }
}
