//! etcd's authentication and authorization API.
//!
//! These API endpoints are used to manage users and roles.

use std::str::FromStr;

use futures::{Future, IntoFuture, Stream};
use hyper::{StatusCode, Uri};
use hyper::client::Connect;
use serde_json;

use async::first_ok;
use client::{Client, ClusterInfo, Response};
use error::{ApiError, Error};

/// The structure returned by the `GET /v2/auth/enable` endpoint.
#[derive(Debug, Deserialize)]
struct AuthStatus {
    /// Whether or not the auth system is enabled.
    pub enabled: bool,
}

/// The result of attempting to disable the auth system.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DisableAuth {
    /// The auth system was already disabled.
    AlreadyDisabled,
    /// The auth system was successfully disabled.
    Disabled,
    /// The attempt to disable the auth system was not done by a root user.
    Unauthorized,
}

impl DisableAuth {
    /// Indicates whether or not the auth system was disabled as a result of the call to `disable`.
    pub fn is_disabled(&self) -> bool {
        match *self {
            DisableAuth::AlreadyDisabled | DisableAuth::Disabled => true,
            DisableAuth::Unauthorized => false,
        }
    }
}

/// The result of attempting to enable the auth system.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EnableAuth {
    /// The auth system was already enabled.
    AlreadyEnabled,
    /// The auth system was successfully enabled.
    Enabled,
    /// The auth system could not be enabled because there is no root user.
    RootUserRequired,
}

impl EnableAuth {
    /// Indicates whether or not the auth system was enabled as a result of the call to `enable`.
    pub fn is_enabled(&self) -> bool {
        match *self {
            EnableAuth::AlreadyEnabled | EnableAuth::Enabled => true,
            EnableAuth::RootUserRequired => false,
        }
    }
}

/// Attempts to disable the auth system.
pub fn disable<C>(
    client: &Client<C>,
) -> Box<Future<Item = Response<DisableAuth>, Error = Vec<Error>>>
where
    C: Clone + Connect,
{
    let http_client = client.http_client().clone();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let url = build_url(member, "/enable");
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| http_client.delete(uri).map_err(Error::from));

        let result = response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());

            let result = match status {
                StatusCode::Ok => Response {
                    data: DisableAuth::Disabled,
                    cluster_info,
                },
                StatusCode::Conflict => Response {
                    data: DisableAuth::AlreadyDisabled,
                    cluster_info,
                },
                StatusCode::Unauthorized => Response {
                    data: DisableAuth::Unauthorized,
                    cluster_info,
                },
                _ => return Err(Error::UnexpectedStatus(status)),
            };

            Ok(result)
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Attempts to enable the auth system.
pub fn enable<C>(
    client: &Client<C>,
) -> Box<Future<Item = Response<EnableAuth>, Error = Vec<Error>>>
where
    C: Clone + Connect,
{
    let http_client = client.http_client().clone();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let url = build_url(member, "/enable");
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| http_client.put(uri, "".to_owned()).map_err(Error::from));

        let result = response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());

            let result = match status {
                StatusCode::Ok => Response {
                    data: EnableAuth::Enabled,
                    cluster_info,
                },
                StatusCode::BadRequest => Response {
                    data: EnableAuth::RootUserRequired,
                    cluster_info,
                },
                StatusCode::Conflict => Response {
                    data: EnableAuth::AlreadyEnabled,
                    cluster_info,
                },
                _ => return Err(Error::UnexpectedStatus(status)),
            };

            Ok(result)
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Determines whether or not the auth system is enabled.
pub fn status<C>(
    client: &Client<C>,
) -> Box<Future<Item = Response<bool>, Error = Vec<Error>>>
where
    C: Clone + Connect,
{
    let http_client = client.http_client().clone();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let url = build_url(member, "/enable");
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| http_client.get(uri).map_err(Error::from));

        let result = response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());
            let body = response.body().concat2().map_err(Error::from);

            body.and_then(move |ref body| if status == StatusCode::Ok {
                match serde_json::from_slice::<AuthStatus>(body) {
                    Ok(data) => Ok(Response {
                        data: data.enabled,
                        cluster_info,
                    }),
                    Err(error) => Err(Error::Serialization(error)),
                }
            } else {
                match serde_json::from_slice::<ApiError>(body) {
                    Ok(error) => Err(Error::Api(error)),
                    Err(error) => Err(Error::Serialization(error)),
                }
            })
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Constructs the full URL for an API call.
fn build_url(endpoint: &Uri, path: &str) -> String {
    let maybe_slash = if endpoint.as_ref().ends_with("/") {
        ""
    } else {
        "/"
    };

    format!("{}{}v2/auth{}", endpoint, maybe_slash, path)
}
