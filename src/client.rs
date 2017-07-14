//! Contains the etcd client. All API calls are made via the client.

use futures::{Future, IntoFuture, Stream};
use futures::future::{FutureResult, err, ok};
use futures::stream::futures_unordered;
use hyper::{Client as Hyper, StatusCode, Uri};
use hyper::client::{Connect, HttpConnector};
#[cfg(feature = "tls")]
use hyper_tls::HttpsConnector;
use serde::de::DeserializeOwned;
use serde_json;
use tokio_core::reactor::Handle;

use error::{ApiError, Error};
use http::HttpClient;
use member::Member;
use version::VersionInfo;

/// API client for etcd. All API calls are made via the client.
#[derive(Clone, Debug)]
pub struct Client<C>
where
    C: Clone + Connect,
{
    http_client: HttpClient<C>,
    members: Vec<Member>,
}

/// A username and password to use for HTTP basic authentication.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BasicAuth {
    /// The username to use for authentication.
    pub username: String,
    /// The password to use for authentication.
    pub password: String,
}

impl Client<HttpConnector> {
    /// Constructs a new client using the HTTP protocol. `endpoints` are URLs for the etcd cluster
    /// members the client will make API calls to.
    ///
    /// # Errors
    ///
    /// Fails if no endpoints are provided or if any of the endpoints is an invalid URL.
    pub fn new(
        handle: &Handle,
        endpoints: &[&str],
        basic_auth: Option<BasicAuth>,
    ) -> Result<Client<HttpConnector>, Error> {
        let hyper = Hyper::configure().keep_alive(true).build(handle);

        Client::custom(hyper, endpoints, basic_auth)
    }
}

#[cfg(feature = "tls")]
impl Client<HttpsConnector<HttpConnector>> {
    /// Constructs a new client using the HTTPS protocol. `endpoints` are URLs for the etcd cluster
    /// members the client will make API calls to.
    ///
    /// # Errors
    ///
    /// Fails if no endpoints are provided or if any of the endpoints is an invalid URL.
    pub fn https(
        handle: &Handle,
        endpoints: &[&str],
        basic_auth: Option<BasicAuth>,
    ) -> Result<Client<HttpsConnector<HttpConnector>>, Error> {
        let connector = HttpsConnector::new(4, handle)?;
        let hyper = Hyper::configure()
            .connector(connector)
            .keep_alive(true)
            .build(handle);

        Client::custom(hyper, endpoints, basic_auth)
    }
}

impl<C> Client<C>
where
    C: Clone + Connect,
{
    /// Constructs a new client using the provided `hyper::Client`. `endpoints` are URLs for the
    /// etcd cluster members the client will make API calls to.
    ///
    /// This method allows the user to configure the details of the underlying HTTP client to their
    /// liking. It is also necessary when using X.509 client certificate authentication.
    ///
    /// # Errors
    ///
    /// Fails if no endpoints are provided or if any of the endpoints is an invalid URL.
    pub fn custom(
        hyper: Hyper<C>,
        endpoints: &[&str],
        basic_auth: Option<BasicAuth>,
    ) -> Result<Client<C>, Error> {
        if endpoints.len() < 1 {
            return Err(Error::NoEndpoints);
        }

        let mut members = Vec::with_capacity(endpoints.len());

        for endpoint in endpoints {
            members.push(Member::new(endpoint)?);
        }

        Ok(Client {
            http_client: HttpClient::new(hyper, basic_auth),
            members: members,
        })
    }

    pub(crate) fn http_client(&self) -> &HttpClient<C> {
        &self.http_client
    }

    pub(crate) fn members(&self) -> &[Member] {
        &self.members
    }

    /// Returns version information from each etcd cluster member the client was initialized with.
    pub fn versions(&self) -> Box<Stream<Item = VersionInfo, Error = Error>> {
        let futures = self.members.iter().map(|member| {
            let url = format!("{}version", member.endpoint);
            let uri = url.parse().map_err(Error::from).into_future();
            let cloned_client = self.http_client.clone();
            let response = uri.and_then(move |uri| cloned_client.get(uri).map_err(Error::from));
            response.and_then(|response| {
                let status = response.status();
                let body = response.body().concat2().map_err(Error::from);

                body.and_then(move |ref body| if status == StatusCode::Ok {
                    match serde_json::from_slice::<VersionInfo>(body) {
                        Ok(stats) => ok(stats),
                        Err(error) => err(Error::Serialization(error)),
                    }
                } else {
                    match serde_json::from_slice::<ApiError>(body) {
                        Ok(error) => err(Error::Api(error)),
                        Err(error) => err(Error::Serialization(error)),
                    }
                })
            })
        });

        Box::new(futures_unordered(futures))
    }

    pub(crate) fn request<T>(
        &self,
        uri: FutureResult<Uri, Error>,
    ) -> Box<Future<Item = T, Error = Error>>
    where
        T: DeserializeOwned + 'static,
    {
        let http_client = self.http_client.clone();
        let response = uri.and_then(move |uri| http_client.get(uri).map_err(Error::from));
        let result = response.and_then(|response| {
            let status = response.status();
            let body = response.body().concat2().map_err(Error::from);

            body.and_then(move |body| if status == StatusCode::Ok {
                match serde_json::from_slice::<T>(&body) {
                    Ok(stats) => ok(stats),
                    Err(error) => err(Error::Serialization(error)),
                }
            } else {
                match serde_json::from_slice::<ApiError>(&body) {
                    Ok(error) => err(Error::Api(error)),
                    Err(error) => err(Error::Serialization(error)),
                }
            })
        });

        Box::new(result)
    }
}
