use hyper::{Client as Hyper, Method, Request, Uri};
use hyper::client::{Connect, FutureResponse};
use hyper::header::{Authorization, Basic, ContentType};

use client::BasicAuth;

#[derive(Clone, Debug)]
pub struct HttpClient<C>
where
    C: Clone + Connect,
{
    basic_auth: Option<BasicAuth>,
    hyper: Hyper<C>,
}

impl<C> HttpClient<C>
where
    C: Clone + Connect,
{
    /// Constructs a new `HttpClient`.
    pub fn new(hyper: Hyper<C>, basic_auth: Option<BasicAuth>) -> Self {
        HttpClient { basic_auth, hyper }
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
        if let Some(ref basic_auth) = self.basic_auth {
            let authorization = Authorization(Basic {
                username: basic_auth.username.clone(),
                password: Some(basic_auth.password.clone()),
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
