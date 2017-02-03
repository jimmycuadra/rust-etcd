use hyper::{Client, Error as HyperError};
use hyper::client::{RequestBuilder, Response};
use hyper::header::{Authorization, Basic, ContentType};
use hyper::method::Method;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use hyper_native_tls::native_tls::TlsConnector;

use client::ClientOptions;
use error::Error;

pub struct HttpClient {
    hyper: Client,
    username_and_password: Option<(String, String)>
}

impl HttpClient {
    /// Constructs a new `HttpClient`.
    pub fn new(options: ClientOptions) -> Result<Self, Error> {
        let mut tls_connector_builder = TlsConnector::builder()?;

        if let Some(pkcs12) = options.pkcs12 {
            tls_connector_builder.identity(pkcs12)?;
        }

        let tls_connector = tls_connector_builder.build()?;
        let native_tls_client = NativeTlsClient::from(tls_connector);
        let connector = HttpsConnector::new(native_tls_client);

        Ok(HttpClient {
            hyper: Client::with_connector(connector),
            username_and_password: options.username_and_password,
        })
    }

    /// Makes a DELETE request to etcd.
    pub fn delete(&self, url: String) -> Result<Response, HyperError> {
        self.request(Method::Delete, url)
    }

    /// Makes a GET request to etcd.
    pub fn get(&self, url: String) -> Result<Response, HyperError> {
        self.request(Method::Get, url)
    }

    /// Makes a POST request to etcd.
    pub fn post(&self, url: String, body: String) -> Result<Response, HyperError> {
        self.request_with_body(Method::Post, url, body)
    }

    /// Makes a PUT request to etcd.
    pub fn put(&self, url: String, body: String) -> Result<Response, HyperError> {
        self.request_with_body(Method::Put, url, body)
    }

    // private

    /// Adds the Authorization HTTP header to a request if a credentials were supplied.
    fn add_auth_header<'a>(&self, request: RequestBuilder<'a>) -> RequestBuilder<'a> {
        if let Some((ref username, ref password)) = self.username_and_password {
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
    fn request(&self, method: Method, url: String) -> Result<Response, HyperError> {
        let mut request = self.hyper.request(method, &url);

        request = self.add_auth_header(request);

        request.send()
    }

    /// Makes a request with an HTTP body to etcd.
    fn request_with_body(&self,
        method: Method,
        url: String,
        body: String,
    ) -> Result<Response, HyperError> {
        let content_type = ContentType::form_url_encoded();
        let mut request = self.hyper.request(method, &url).header(content_type).body(&body);

        request = self.add_auth_header(request);

        request.send()
    }
}
