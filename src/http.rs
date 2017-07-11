use hyper::{Body, Client, Method, Request, Uri};
use hyper::client::{FutureResponse, HttpConnector};
use hyper::header::{Authorization, Basic, ContentType};
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Handle;

use client::ClientOptions;
use error::Error;

#[derive(Clone, Debug)]
pub struct HttpClient {
    hyper: Client<HttpsConnector<HttpConnector>, Body>,
    username_and_password: Option<(String, String)>
}

impl HttpClient {
    /// Constructs a new `HttpClient`.
    pub fn new(handle: &Handle, options: ClientOptions) -> Result<Self, Error> {
        let connector = match options.tls_connector {
            Some(tls_connector) => {
                let mut http_connector = HttpConnector::new(4, handle);

                http_connector.enforce_http(false);

                HttpsConnector::from((http_connector, tls_connector))
            }
            None => HttpsConnector::new(4, handle)?,
        };

        let hyper = Client::configure().connector(connector).build(handle);

        Ok(HttpClient {
            hyper: hyper,
            username_and_password: options.username_and_password,
        })
    }

    /// Makes a DELETE request to etcd.
    pub fn delete(&self, uri: Uri) -> FutureResponse {
        self.request(Method::Delete, uri)
    }

    /// Makes a GET request to etcd.
    pub fn get(&self, uri: Uri) -> FutureResponse {
        self.request(Method::Get, uri)
    }

    /// Makes a POST request to etcd.
    pub fn post(&self, uri: Uri, body: String) -> FutureResponse {
        self.request_with_body(Method::Post, uri, body)
    }

    /// Makes a PUT request to etcd.
    pub fn put(&self, uri: Uri, body: String) -> FutureResponse {
        self.request_with_body(Method::Put, uri, body)
    }

    // private

    /// Adds the Authorization HTTP header to a request if a credentials were supplied.
    fn add_auth_header<'a>(&self, request: &mut Request) {
        if let Some((ref username, ref password)) = self.username_and_password {
            let authorization = Authorization(Basic {
                username: username.clone(),
                password: Some(password.clone()),
            });

            request.headers_mut().set(authorization);
        }
    }

    /// Makes a request to etcd.
    fn request(&self, method: Method, uri: Uri) -> FutureResponse {
        let mut request = Request::new(method, uri);

        self.add_auth_header(&mut request);

        self.hyper.request(request)
    }

    /// Makes a request with an HTTP body to etcd.
    fn request_with_body(&self, method: Method, uri: Uri, body: String) -> FutureResponse {
        let content_type = ContentType::form_url_encoded();

        let mut request = Request::new(method, uri);
        request.headers_mut().set(content_type);
        request.set_body(body);

        self.add_auth_header(&mut request);

        self.hyper.request(request)
    }
}
