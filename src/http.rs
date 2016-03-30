use std::sync::Arc;

use hyper::{Client, Error};
use hyper::client::{RequestBuilder, Response};
use hyper::header::{Authorization, Basic, ContentType};
use hyper::method::Method;
use hyper::net::{HttpsConnector, Openssl};
use openssl::ssl::{SSL_VERIFY_PEER, SslContext, SslMethod};
use openssl::x509::X509FileType;

use client::ClientOptions;

#[derive(Debug)]
pub struct HttpClient {
    hyper: Client,
    options: ClientOptions,
}

impl HttpClient {
    /// Constructs a new `HttpClient`.
    pub fn new(options: ClientOptions) -> Result<Self, Error> {
        if options.ca.is_some() || options.cert_and_key.is_some() {
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
                hyper: Client::with_connector(connector),
                options: options,
            })
        } else {
            Ok(HttpClient {
                hyper: Client::new(),
                options: options,
            })
        }
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

    /// Adds the Authorization HTTP header to a request if a credentials were supplied.
    fn add_auth_header<'a>(&self, request: RequestBuilder<'a>) -> RequestBuilder<'a> {
        if let Some((ref username, ref password)) = self.options.username_and_password {
            let authorization = Authorization(Basic {
                username: username.clone(),
                password: Some(password.clone()),
            });

            request.header(authorization)
        } else {
            request
        }
    }

    /// Makes a request to etcd.
    fn request(&self, method: Method, url: String) -> Result<Response, Error> {
        let mut request = self.hyper.request(method, &url);

        request = self.add_auth_header(request);

        request.send()
    }

    /// Makes a request with an HTTP body to etcd.
    fn request_with_body(&self,
        method: Method,
        url: String,
        body: String,
    ) -> Result<Response, Error> {
        let content_type = ContentType::form_url_encoded();
        let mut request = self.hyper.request(method, &url).header(content_type).body(&body);

        request = self.add_auth_header(request);

        request.send()
    }
}
