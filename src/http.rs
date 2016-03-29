use std::sync::Arc;

use hyper::{Client, Error};
use hyper::client::Response;
use hyper::header::ContentType;
use hyper::method::Method;
use hyper::net::{HttpsConnector, Openssl};
use openssl::ssl::{SSL_VERIFY_PEER, SslContext, SslMethod};
use openssl::x509::X509FileType;

use client::SslOptions;

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

    pub fn https(options: &SslOptions) -> Result<Self, Error> {
        let mut ctx = try!(SslContext::new(SslMethod::Sslv23));

        if let Some(ref ca) = options.ca {
            try!(ctx.set_CA_file(ca));
        }

        if let Some((ref cert, ref key)) = options.cert_and_key {
            try!(ctx.set_certificate_file(cert, X509FileType::PEM));
            try!(ctx.set_private_key_file(key, X509FileType::PEM));
        }

        ctx.set_verify(SSL_VERIFY_PEER, None);

        let openssl = Openssl { context: Arc::new(ctx) };
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
