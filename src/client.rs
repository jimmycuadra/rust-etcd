//! Contains the etcd client. All API calls are made via the client.

use futures::{Future, IntoFuture, Stream};
use futures::future::{FutureResult, err, ok};
use futures::stream::futures_unordered;
use hyper::{Client as Hyper, Headers, StatusCode, Uri};
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

header! {
    /// The `X-Etcd-Cluster-Id` header.
    (XEtcdClusterId, "X-Etcd-Cluster-Id") => [String]
}

header! {
    /// The `X-Etcd-Index` HTTP header.
    (XEtcdIndex, "X-Etcd-Index") => [u64]
}

header! {
    /// The `X-Raft-Index` HTTP header.
    (XRaftIndex, "X-Raft-Index") => [u64]
}

header! {
    /// The `X-Raft-Term` HTTP header.
    (XRaftTerm, "X-Raft-Term") => [u64]
}

/// API client for etcd.
///
/// All API calls require a client.
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
    /// Constructs a new client using the HTTP protocol.
    ///
    /// # Parameters
    ///
    /// * handle: A handle to the event loop.
    /// * endpoints: URLs for one or more cluster members. When making an API call, the client will
    /// make the call to each member in order until it receives a successful respponse.
    /// * basic_auth: Credentials for HTTP basic authentication.
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
    /// Constructs a new client using the HTTPS protocol.
    ///
    /// # Parameters
    ///
    /// * handle: A handle to the event loop.
    /// * endpoints: URLs for one or more cluster members. When making an API call, the client will
    /// make the call to each member in order until it receives a successful respponse.
    /// * basic_auth: Credentials for HTTP basic authentication.
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
    /// Constructs a new client using the provided `hyper::Client`.
    ///
    /// This method allows the user to configure the details of the underlying HTTP client to their
    /// liking. It is also necessary when using X.509 client certificate authentication.
    ///
    /// # Parameters
    ///
    /// * hyper: A fully configured `hyper::Client`.
    /// * endpoints: URLs for one or more cluster members. When making an API call, the client will
    /// make the call to each member in order until it receives a successful respponse.
    /// * basic_auth: Credentials for HTTP basic authentication.
    ///
    /// # Errors
    ///
    /// Fails if no endpoints are provided or if any of the endpoints is an invalid URL.
    ///
    /// # Examples
    ///
    /// Configuring the client to authenticate with both HTTP basic auth and an X.509 client
    /// certificate:
    ///
    /// ```no_run
    /// extern crate etcd;
    /// extern crate futures;
    /// extern crate hyper;
    /// extern crate hyper_tls;
    /// extern crate native_tls;
    /// extern crate tokio_core;
    ///
    /// use std::fs::File;
    /// use std::io::Read;
    ///
    /// use futures::Future;
    /// use hyper::client::HttpConnector;
    /// use hyper_tls::HttpsConnector;
    /// use native_tls::{Certificate, Pkcs12, TlsConnector};
    /// use tokio_core::reactor::Core;
    ///
    /// use etcd::{Client, kv};
    ///
    /// fn main() {
    ///     let mut ca_cert_file = File::open("ca.der").unwrap();
    ///     let mut ca_cert_buffer = Vec::new();
    ///     ca_cert_file.read_to_end(&mut ca_cert_buffer).unwrap();
    ///
    ///     let mut pkcs12_file = File::open("/source/tests/ssl/client.p12").unwrap();
    ///     let mut pkcs12_buffer = Vec::new();
    ///     pkcs12_file.read_to_end(&mut pkcs12_buffer).unwrap();
    ///
    ///     let mut builder = TlsConnector::builder().unwrap();
    ///     builder.add_root_certificate(Certificate::from_der(&ca_cert_buffer).unwrap()).unwrap();
    ///     builder.identity(Pkcs12::from_der(&pkcs12_buffer, "secret").unwrap()).unwrap();
    ///
    ///     let tls_connector = builder.build().unwrap();
    ///
    ///     let mut core = Core::new().unwrap();
    ///     let handle = core.handle();
    ///
    ///     let mut http_connector = HttpConnector::new(4, &handle);
    ///     http_connector.enforce_http(false);
    ///     let https_connector = HttpsConnector::from((http_connector, tls_connector));
    ///
    ///     let hyper = hyper::Client::configure().connector(https_connector).build(&handle);
    ///
    ///     let client = Client::custom(hyper, &["https://etcd.example.com:2379"], None).unwrap();
    ///
    ///     let work = kv::set(&client, "/foo", "bar", None).and_then(|_| {
    ///         kv::get(&client, "/foo", kv::GetOptions::default())
    ///             .and_then(|(key_value_info, _)| {
    ///                 let value = key_value_info.node.value.unwrap();
    ///
    ///                 assert_eq!(value, "bar".to_string());
    ///
    ///                 Ok(())
    ///             })
    ///     });
    ///
    ///     core.run(work).unwrap();
    /// }
    /// ```
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

    /// Lets other internal code access the `HttpClient`.
    pub(crate) fn http_client(&self) -> &HttpClient<C> {
        &self.http_client
    }

    /// Lets other internal code access the cluster `Member`s.
    pub(crate) fn members(&self) -> &[Member] {
        &self.members
    }

    /// Returns version information from each etcd cluster member the client was initialized with.
    pub fn versions(&self) -> Box<Stream<Item = VersionInfo, Error = Error>> {
        let futures = self.members.iter().map(|member| {
            let url = build_url(&member);
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

    /// Lets other internal code make basic HTTP requests.
    pub(crate) fn request<T>(
        &self,
        uri: FutureResult<Uri, Error>,
    ) -> Box<Future<Item = (T, ClusterInfo), Error = Error>>
    where
        T: DeserializeOwned + 'static,
    {
        let http_client = self.http_client.clone();
        let response = uri.and_then(move |uri| http_client.get(uri).map_err(Error::from));
        let result = response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());
            let body = response.body().concat2().map_err(Error::from);

            body.and_then(move |body| if status == StatusCode::Ok {
                match serde_json::from_slice::<T>(&body) {
                    Ok(stats) => ok((stats, cluster_info)),
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

/// Information about the state of the etcd cluster from an API response's HTTP headers.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ClusterInfo {
    /// An internal identifier for the cluster.
    pub cluster_id: Option<String>,
    /// A unique, monotonically-incrementing integer created for each change to etcd.
    pub etcd_index: Option<u64>,
    /// A unique, monotonically-incrementing integer used by the Raft protocol.
    pub raft_index: Option<u64>,
    /// The current Raft election term.
    pub raft_term: Option<u64>,
}

impl<'a> From<&'a Headers> for ClusterInfo {
    fn from(headers: &'a Headers) -> Self {

        let cluster_id = match headers.get::<XEtcdClusterId>() {
            Some(&XEtcdClusterId(ref value)) => Some(value.clone()),
            None => None,
        };
        let etcd_index = match headers.get::<XEtcdIndex>() {
            Some(&XEtcdIndex(value)) => Some(value),
            None => None,
        };

        let raft_index = match headers.get::<XRaftIndex>() {
            Some(&XRaftIndex(value)) => Some(value),
            None => None,
        };

        let raft_term = match headers.get::<XRaftTerm>() {
            Some(&XRaftTerm(value)) => Some(value),
            None => None,
        };

        ClusterInfo {
            cluster_id,
            etcd_index,
            raft_index,
            raft_term,
        }
    }
}

/// Constructs the full URL for the versions API call.
fn build_url(member: &Member) -> String {
    let maybe_slash = if member.endpoint.as_ref().ends_with("/") {
        ""
    } else {
        "/"
    };

    format!("{}{}version", member.endpoint, maybe_slash)
}
