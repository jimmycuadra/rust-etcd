//! etcd's members API.
//!
//! These API endpoints are used to manage cluster membership.

use std::str::FromStr;

use futures::{Future, IntoFuture, Stream};
use hyper::{StatusCode, Uri};
use hyper::client::Connect;
use serde_json;

use async::first_ok;
use client::{Client, ClusterInfo};
use error::{ApiError, Error};
use member::Member as SimpleMember;

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

/// Lists the members of the cluster.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
pub fn list<C>(
    client: &Client<C>,
) -> Box<Future<Item = (Vec<Member>, ClusterInfo), Error = Vec<Error>>>
where
    C: Clone + Connect,
{
    let http_client = client.http_client().clone();

    let result = first_ok(client.members().to_vec(), move |member| {
        let url = build_url(member, "");
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
                match serde_json::from_slice::<Vec<Member>>(body) {
                    Ok(members) => Ok((members, cluster_info)),
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
fn build_url(member: &SimpleMember, path: &str) -> String {
    let maybe_slash = if member.endpoint.as_ref().ends_with("/") {
        ""
    } else {
        "/"
    };

    format!("{}{}v2/members{}", member.endpoint, maybe_slash, path)
}
