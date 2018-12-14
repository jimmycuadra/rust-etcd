//! Contains the etcd client. All API calls are made via the client.

use futures::stream::futures_unordered;
use futures::{Future, IntoFuture, Stream};
use hyper::client::connect::{Connect, HttpConnector};
use hyper::{Client as Hyper, StatusCode, Uri};
use crate::hyper_http::header::{HeaderMap, HeaderValue};
#[cfg(feature = "tls")]
use hyper_tls::HttpsConnector;
use serde::de::DeserializeOwned;
use serde_json;
// use tokio_core::reactor::Handle;

use crate::error::{ApiError, Error};
use crate::http::HttpClient;
use crate::version::VersionInfo;

// header! {
//     /// The `X-Etcd-Cluster-Id` header.
//     (XEtcdClusterId, "X-Etcd-Cluster-Id") => [String]
// }
const XETCD_CLUSTER_ID: &str = "X-Etcd-Cluster-Id";

// header! {
//     /// The `X-Etcd-Index` HTTP header.
//     (XEtcdIndex, "X-Etcd-Index") => [u64]
// }
const XETCD_INDEX: &str = "X-Etcd-Index";

// header! {
//     /// The `X-Raft-Index` HTTP header.
//     (XRaftIndex, "X-Raft-Index") => [u64]
// }
const XRAFT_INDEX: &str = "X-Raft-Index";

// header! {
//     /// The `X-Raft-Term` HTTP header.
//     (XRaftTerm, "X-Raft-Term") => [u64]
// }
const XRAFT_TERM: &str = "X-Raft-Term";

/// API client for etcd.
///
/// All API calls require a client.
#[derive(Clone, Debug)]
pub struct Client<C>
where
    C: Clone + Connect + Sync + 'static,
{
    endpoints: Vec<Uri>,
    http_client: HttpClient<C>,
}

/// A username and password to use for HTTP basic authentication.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BasicAuth {
    /// The username to use for authentication.
    pub username: String,
    /// The password to use for authentication.
    pub password: String,
}

/// A value returned by the health check API endpoint to indicate a healthy cluster member.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct Health {
    /// The health status of the cluster member.
    pub health: String,
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
        endpoints: &[&str],
        basic_auth: Option<BasicAuth>,
    ) -> Result<Client<HttpConnector>, Error> {
        let hyper = Hyper::builder().keep_alive(true).build_http();

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
        endpoints: &[&str],
        basic_auth: Option<BasicAuth>,
    ) -> Result<Client<HttpsConnector<HttpConnector>>, Error> {
        let connector = HttpsConnector::new(4)?;
        let hyper = Hyper::builder().keep_alive(true).build(connector);

        Client::custom(hyper, endpoints, basic_auth)
    }
}

impl<C> Client<C>
where
    C: Clone + Connect + Sync + 'static,
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
    /// use native_tls::{Certificate, TlsConnector, Identity};
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
    ///     let mut builder = TlsConnector::builder();
    ///     builder.add_root_certificate(Certificate::from_der(&ca_cert_buffer).unwrap());
    ///     builder.identity(Identity::from_pkcs12(&pkcs12_buffer, "secret").unwrap());
    ///
    ///     let tls_connector = builder.build().unwrap();
    ///
    ///     let mut core = Core::new().unwrap();
    ///
    ///     let mut http_connector = HttpConnector::new(4);
    ///     http_connector.enforce_http(false);
    ///     let https_connector = HttpsConnector::from((http_connector, tls_connector));
    ///
    ///     let hyper = hyper::Client::builder().build(https_connector);
    ///
    ///     let client = Client::custom(hyper, &["https://etcd.example.com:2379"], None).unwrap();
    ///
    ///     let work = kv::set(&client, "/foo", "bar", None).and_then(|_| {
    ///         let get_request = kv::get(&client, "/foo", kv::GetOptions::default());
    ///
    ///         get_request.and_then(|response| {
    ///             let value = response.data.node.value.unwrap();
    ///
    ///             assert_eq!(value, "bar".to_string());
    ///
    ///             Ok(())
    ///         })
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

        let mut uri_endpoints = Vec::with_capacity(endpoints.len());

        for endpoint in endpoints {
            uri_endpoints.push(endpoint.parse()?);
        }

        Ok(Client {
            endpoints: uri_endpoints,
            http_client: HttpClient::new(hyper, basic_auth),
        })
    }

    /// Lets other internal code access the `HttpClient`.
    pub(crate) fn http_client(&self) -> &HttpClient<C> {
        &self.http_client
    }

    /// Lets other internal code access the cluster endpoints.
    pub(crate) fn endpoints(&self) -> &[Uri] {
        &self.endpoints
    }

    /// Runs a basic health check against each etcd member.
    pub fn health(&self) -> impl Stream<Item = Response<Health>, Error = Error> {
        let futures = self.endpoints.iter().map(|endpoint| {
            let url = build_url(&endpoint, "health");
            let uri = url.parse().map_err(Error::from).into_future();
            let cloned_client = self.http_client.clone();
            let response = uri.and_then(move |uri| cloned_client.get(uri).map_err(Error::from));
            response.and_then(|response| {
                let status = response.status();
                let cluster_info = ClusterInfo::from(response.headers());
                let body = response.into_body().concat2().map_err(Error::from);

                body.and_then(move |ref body| {
                    if status == StatusCode::OK {
                        match serde_json::from_slice::<Health>(body) {
                            Ok(data) => Ok(Response { data, cluster_info }),
                            Err(error) => Err(Error::Serialization(error)),
                        }
                    } else {
                        match serde_json::from_slice::<ApiError>(body) {
                            Ok(error) => Err(Error::Api(error)),
                            Err(error) => Err(Error::Serialization(error)),
                        }
                    }
                })
            })
        });

        futures_unordered(futures)
    }

    /// Returns version information from each etcd cluster member the client was initialized with.
    pub fn versions(&self) -> impl Stream<Item = Response<VersionInfo>, Error = Error> {
        let futures = self.endpoints.iter().map(|endpoint| {
            let url = build_url(&endpoint, "version");
            let uri = url.parse().map_err(Error::from).into_future();
            let cloned_client = self.http_client.clone();
            let response = uri.and_then(move |uri| cloned_client.get(uri).map_err(Error::from));
            response.and_then(|response| {
                let status = response.status();
                let cluster_info = ClusterInfo::from(response.headers());
                let body = response.into_body().concat2().map_err(Error::from);

                body.and_then(move |ref body| {
                    if status == StatusCode::OK {
                        match serde_json::from_slice::<VersionInfo>(body) {
                            Ok(data) => Ok(Response { data, cluster_info }),
                            Err(error) => Err(Error::Serialization(error)),
                        }
                    } else {
                        match serde_json::from_slice::<ApiError>(body) {
                            Ok(error) => Err(Error::Api(error)),
                            Err(error) => Err(Error::Serialization(error)),
                        }
                    }
                })
            })
        });

        futures_unordered(futures)
    }

    /// Lets other internal code make basic HTTP requests.
    pub(crate) fn request<U, T>(&self, uri: U) -> impl Future<Item = Response<T>, Error = Error>
    where
        U: Future<Item = Uri, Error = Error>,
        T: DeserializeOwned + 'static,
    {
        let http_client = self.http_client.clone();
        let response = uri.and_then(move |uri| http_client.get(uri).map_err(Error::from));
        response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());
            let body = response.into_body().concat2().map_err(Error::from);

            body.and_then(move |body| {
                if status == StatusCode::OK {
                    match serde_json::from_slice::<T>(&body) {
                        Ok(data) => Ok(Response { data, cluster_info }),
                        Err(error) => Err(Error::Serialization(error)),
                    }
                } else {
                    match serde_json::from_slice::<ApiError>(&body) {
                        Ok(error) => Err(Error::Api(error)),
                        Err(error) => Err(Error::Serialization(error)),
                    }
                }
            })
        })
    }
}

/// A wrapper type returned by all API calls.
///
/// Contains the primary data of the response along with information about the cluster extracted
/// from the HTTP response headers.
#[derive(Clone, Debug)]
pub struct Response<T> {
    /// Information about the state of the cluster.
    pub cluster_info: ClusterInfo,
    /// The primary data of the response.
    pub data: T,
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

impl<'a> From<&'a HeaderMap<HeaderValue>> for ClusterInfo {
    fn from(headers: &'a HeaderMap<HeaderValue>) -> Self {
        let cluster_id = headers.get(XETCD_CLUSTER_ID).and_then(|v| {
            match String::from_utf8(v.as_bytes().to_vec()) {
                Ok(s) => Some(s),
                Err(e) => {
                    error!("{} header decode error: {:?}", XETCD_CLUSTER_ID, e);
                    None
                }
            }
        });

        let etcd_index = headers.get(XETCD_INDEX).and_then(|v| {
            match String::from_utf8(v.as_bytes().to_vec())
                .map_err(|e| format!("{:?}", e))
                .and_then(|s| s.parse().map_err(|e| format!("{:?}", e)))
            {
                Ok(i) => Some(i),
                Err(e) => {
                    error!("{} header decode error: {}", XETCD_INDEX, e);
                    None
                }
            }
        });

        let raft_index = headers.get(XRAFT_INDEX).and_then(|v| {
            match String::from_utf8(v.as_bytes().to_vec())
                .map_err(|e| format!("{:?}", e))
                .and_then(|s| s.parse().map_err(|e| format!("{:?}", e)))
            {
                Ok(i) => Some(i),
                Err(e) => {
                    error!("{} header decode error: {}", XRAFT_INDEX, e);
                    None
                }
            }
        });

        let raft_term = headers.get(XRAFT_TERM).and_then(|v| {
            match String::from_utf8(v.as_bytes().to_vec())
                .map_err(|e| format!("{:?}", e))
                .and_then(|s| s.parse().map_err(|e| format!("{:?}", e)))
            {
                Ok(i) => Some(i),
                Err(e) => {
                    error!("{} header decode error: {}", XRAFT_TERM, e);
                    None
                }
            }
        });

        ClusterInfo {
            cluster_id: cluster_id,
            etcd_index: etcd_index,
            raft_index: raft_index,
            raft_term: raft_term,
        }
    }
}

/// Constructs the full URL for the versions API call.
fn build_url(endpoint: &Uri, path: &str) -> String {
    format!("{}{}", endpoint, path)
}
