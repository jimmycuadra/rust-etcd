//! etcd's members API.
//!
//! These API endpoints are used to manage cluster membership.

use std::str::FromStr;

use futures::{Future, IntoFuture, Stream};
use hyper::client::connect::Connect;
use hyper::{StatusCode, Uri};
use serde_json;

use async::first_ok;
use client::{Client, ClusterInfo, Response};
use error::{ApiError, Error};

/// An etcd server that is a member of a cluster.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct Member {
    /// An internal identifier for the cluster member.
    pub id: String,
    /// A human-readable name for the cluster member.
    pub name: String,
    /// URLs exposing this cluster member's peer API.
    #[serde(rename = "peerURLs")]
    pub peer_urls: Vec<String>,
    /// URLs exposing this cluster member's client API.
    #[serde(rename = "clientURLs")]
    pub client_urls: Vec<String>,
}

/// The request body for `POST /v2/members` and `PUT /v2/members/:id`.
#[derive(Debug, Serialize)]
struct PeerUrls {
    /// The peer URLs.
    #[serde(rename = "peerURLs")]
    peer_urls: Vec<String>,
}

/// A small wrapper around `Member` to match the response of `GET /v2/members`.
#[derive(Debug, Deserialize)]
struct ListResponse {
    /// The members.
    members: Vec<Member>,
}

/// Adds a new member to the cluster.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * peer_urls: URLs exposing this cluster member's peer API.
pub fn add<C>(
    client: &Client<C>,
    peer_urls: Vec<String>,
) -> Box<Future<Item = Response<()>, Error = Vec<Error>>>
where
    C: Clone + Connect,
{
    let peer_urls = PeerUrls { peer_urls };

    let body = match serde_json::to_string(&peer_urls) {
        Ok(body) => body,
        Err(error) => return Box::new(Err(vec![Error::Serialization(error)]).into_future()),
    };

    let http_client = client.http_client().clone();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let url = build_url(member, "");
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let body = body.clone();
        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| http_client.post(uri, body).map_err(Error::from));

        response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());
            let body = response.into_body().concat2().map_err(Error::from);

            body.and_then(move |ref body| {
                if status == StatusCode::CREATED {
                    Ok(Response {
                        data: (),
                        cluster_info,
                    })
                } else {
                    match serde_json::from_slice::<ApiError>(body) {
                        Ok(error) => Err(Error::Api(error)),
                        Err(error) => Err(Error::Serialization(error)),
                    }
                }
            })
        })
    });

    Box::new(result)
}

/// Deletes a member from the cluster.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * id: The unique identifier of the member to delete.
pub fn delete<C>(
    client: &Client<C>,
    id: String,
) -> impl Future<Item = Response<()>, Error = Vec<Error>>
where
    C: Clone + Connect,
{
    let http_client = client.http_client().clone();

    first_ok(client.endpoints().to_vec(), move |member| {
        let url = build_url(member, &format!("/{}", id));
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| http_client.delete(uri).map_err(Error::from));

        response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());
            let body = response.into_body().concat2().map_err(Error::from);

            body.and_then(move |ref body| {
                if status == StatusCode::NO_CONTENT {
                    Ok(Response {
                        data: (),
                        cluster_info,
                    })
                } else {
                    match serde_json::from_slice::<ApiError>(body) {
                        Ok(error) => Err(Error::Api(error)),
                        Err(error) => Err(Error::Serialization(error)),
                    }
                }
            })
        })
    })
}

/// Lists the members of the cluster.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
pub fn list<C>(client: &Client<C>) -> impl Future<Item = Response<Vec<Member>>, Error = Vec<Error>>
where
    C: Clone + Connect,
{
    let http_client = client.http_client().clone();

    first_ok(client.endpoints().to_vec(), move |member| {
        let url = build_url(member, "");
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| http_client.get(uri).map_err(Error::from));

        response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());
            let body = response.into_body().concat2().map_err(Error::from);

            body.and_then(move |ref body| {
                if status == StatusCode::OK {
                    match serde_json::from_slice::<ListResponse>(body) {
                        Ok(data) => Ok(Response {
                            data: data.members,
                            cluster_info,
                        }),
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
    })
}

/// Updates the peer URLs of a member of the cluster.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * id: The unique identifier of the member to update.
/// * peer_urls: URLs exposing this cluster member's peer API.
pub fn update<C>(
    client: &Client<C>,
    id: String,
    peer_urls: Vec<String>,
) -> Box<Future<Item = Response<()>, Error = Vec<Error>>>
where
    C: Clone + Connect,
{
    let peer_urls = PeerUrls { peer_urls };

    let body = match serde_json::to_string(&peer_urls) {
        Ok(body) => body,
        Err(error) => return Box::new(Err(vec![Error::Serialization(error)]).into_future()),
    };

    let http_client = client.http_client().clone();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let url = build_url(member, &format!("/{}", id));
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let body = body.clone();
        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| http_client.put(uri, body).map_err(Error::from));

        response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());
            let body = response.into_body().concat2().map_err(Error::from);

            body.and_then(move |ref body| {
                if status == StatusCode::NO_CONTENT {
                    Ok(Response {
                        data: (),
                        cluster_info,
                    })
                } else {
                    match serde_json::from_slice::<ApiError>(body) {
                        Ok(error) => Err(Error::Api(error)),
                        Err(error) => Err(Error::Serialization(error)),
                    }
                }
            })
        })
    });

    Box::new(result)
}

/// Constructs the full URL for an API call.
fn build_url(endpoint: &Uri, path: &str) -> String {
    format!("{}v2/members{}", endpoint, path)
}
