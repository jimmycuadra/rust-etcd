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

/// The type returned when the auth system is successfully enabled or disabled.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AuthChange {
    /// The auth system was successfully enabled or disabled.
    Changed,
    /// The auth system was already in the desired state.
    Unchanged,
}

/// An existing etcd user.
#[derive(Debug, Clone, Deserialize, Eq, Hash, PartialEq)]
pub struct User {
    /// The user's name.
    name: String,
    /// Roles granted to the user.
    roles: Vec<Role>,
}

impl User {
    /// Returns the user's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the roles granted to the user.
    pub fn roles(&self) -> &[Role] {
        &self.roles
    }
}

/// A list of all users.
#[derive(Debug, Clone, Deserialize, Eq, Hash, PartialEq)]
struct Users {
    users: Option<Vec<User>>,
}

/// Paramters used to create a new etcd user.
#[derive(Debug, Clone, Eq, Hash, PartialEq, Serialize)]
pub struct NewUser {
    /// The user's name.
    name: String,
    /// The user's password.
    password: String,
    /// An initial set of roles granted to the user.
    roles: Option<Vec<String>>,
}

impl NewUser {
    /// Creates a new user.
    pub fn new<N, P>(name: N, password: P) -> Self
    where
        N: Into<String>,
        P: Into<String>,
    {
        NewUser {
            name: name.into(),
            password: password.into(),
            roles: None,
        }
    }

    /// Gets the name of the new user.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Grants a role to the new user.
    pub fn add_role<R>(&mut self, role: R)
    where
        R: Into<String>,
    {
        match self.roles {
            Some(ref mut roles) => roles.push(role.into()),
            None => self.roles = Some(vec![role.into()]),
        }
    }
}

/// Parameters used to update an existing etcd user.
#[derive(Debug, Clone, Eq, Hash, PartialEq, Serialize)]
pub struct UserUpdate {
    /// The user's name.
    name: String,
    /// A new password for the user.
    password: Option<String>,
    /// Roles being granted to the user.
    #[serde(rename = "grant")]
    grants: Option<Vec<String>>,
    /// Roles being revoked from the user.
    #[serde(rename = "revoke")]
    revocations: Option<Vec<String>>,
}

impl UserUpdate {
    /// Creates a new `UserUpdate` for the given user.
    pub fn new<N>(name: N) -> Self
    where
        N: Into<String>,
    {
        UserUpdate {
            name: name.into(),
            password: None,
            grants: None,
            revocations: None,
        }
    }

    /// Gets the name of the user being updated.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Updates the user's password.
    pub fn update_password<P>(&mut self, password: P)
    where
        P: Into<String>,
    {
        self.password = Some(password.into());
    }

    /// Grants the given role to the user.
    pub fn grant_role<R>(&mut self, role: R)
    where
        R: Into<String>,
    {
        match self.grants {
            Some(ref mut grants) => grants.push(role.into()),
            None => self.grants = Some(vec![role.into()]),
        }
    }

    /// Revokes the given role from the user.
    pub fn revoke_role<R>(&mut self, role: R)
    where
        R: Into<String>,
    {
        match self.revocations {
            Some(ref mut revocations) => revocations.push(role.into()),
            None => self.revocations = Some(vec![role.into()]),
        }
    }
}

/// An authorization role.
#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq, Serialize)]
pub struct Role {
    /// The name of the role.
    name: String,
    /// Permissions granted to the role.
    permissions: Permissions,
}

impl Role {
    /// Creates a new role.
    pub fn new<N>(name: N) -> Self
    where
        N: Into<String>,
    {
        Role {
            name: name.into(),
            permissions: Permissions::new(),
        }
    }

    /// Gets the name of the role.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Grants read permission for a key in etcd's key-value store to this role.
    pub fn grant_kv_read_permission<K>(&mut self, key: K)
    where
        K: Into<String>,
    {
        self.permissions.kv.grant_read_permission(key)
    }

    /// Grants write permission for a key in etcd's key-value store to this role.
    pub fn grant_kv_write_permission<K>(&mut self, key: K)
    where
        K: Into<String>,
    {
        self.permissions.kv.grant_write_permission(key)
    }

    /// Returns a list of keys in etcd's key-value store that this role is allowed to read.
    pub fn kv_read_permissions(&self) -> &[String] {
        &self.permissions.kv.read
    }

    /// Returns a list of keys in etcd's key-value store that this role is allowed to write.
    pub fn kv_write_permissions(&self) -> &[String] {
        &self.permissions.kv.write
    }
}

/// A list of all roles.
#[derive(Debug, Clone, Deserialize, Eq, Hash, PartialEq)]
struct Roles {
    roles: Option<Vec<Role>>,
}

/// Parameters used to update an existing authorization role.
#[derive(Debug, Clone, Eq, Hash, PartialEq, Serialize)]
pub struct RoleUpdate {
    /// The name of the role.
    name: String,
    /// Permissions being added to the role.
    #[serde(rename = "grant")]
    grants: Permissions,
    /// Permissions being removed from the role.
    #[serde(rename = "revoke")]
    revocations: Permissions,
}

impl RoleUpdate {
    /// Creates a new `RoleUpdate` for the given role.
    pub fn new<R>(role: R) -> Self
    where
        R: Into<String>,
    {
        RoleUpdate {
            name: role.into(),
            grants: Permissions::new(),
            revocations: Permissions::new(),
        }
    }

    /// Gets the name of the role being updated.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Grants read permission for a key in etcd's key-value store to this role.
    pub fn grant_kv_read_permission<K>(&mut self, key: K)
    where
        K: Into<String>,
    {
        self.grants.kv.grant_read_permission(key)
    }

    /// Grants write permission for a key in etcd's key-value store to this role.
    pub fn grant_kv_write_permission<K>(&mut self, key: K)
    where
        K: Into<String>,
    {
        self.grants.kv.grant_write_permission(key)
    }

    /// Revokes read permission for a key in etcd's key-value store from this role.
    pub fn revoke_kv_read_permission<K>(&mut self, key: &K)
    where
        K: Into<String>,
        String: PartialEq<K>,
    {
        self.revocations.kv.revoke_read_permission(key)
    }

    /// Revokes write permission for a key in etcd's key-value store from this role.
    pub fn revoke_kv_write_permission<K>(&mut self, key: &K)
    where
        K: Into<String>,
        String: PartialEq<K>,
    {
        self.revocations.kv.revoke_write_permission(key)
    }
}

/// The access permissions granted to a role.
#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq, Serialize)]
struct Permissions {
    /// Permissions for etcd's key-value store.
    kv: Permission,
}

impl Permissions {
    /// Creates a new set of permissions.
    fn new() -> Self {
        Permissions {
            kv: Permission::new(),
        }
    }
}

/// A set of read and write access permissions for etcd resources.
#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq, Serialize)]
struct Permission {
    /// Resources allowed to be read.
    read: Vec<String>,
    /// Resources allowed to be written.
    write: Vec<String>,
}

impl Permission {
    /// Creates a new permission record.
    fn new() -> Self {
        Permission {
            read: Vec::new(),
            write: Vec::new(),
        }
    }

    /// Grants read access to a resource.
    fn grant_read_permission<K>(&mut self, key: K)
    where
        K: Into<String>,
    {
        self.read.push(key.into())
    }

    /// Grants write access to a resource.
    fn grant_write_permission<K>(&mut self, key: K)
    where
        K: Into<String>,
    {
        self.write.push(key.into())
    }

    /// Revokes read access to a resource.
    fn revoke_read_permission<K>(&mut self, key: &K)
    where
        K: Into<String>,
        String: PartialEq<K>,
    {
        if let Some(position) = self.read.iter().position(|k| k == key) {
            self.read.remove(position);
        }
    }

    /// Revokes write access to a resource.
    fn revoke_write_permission<K>(&mut self, key: &K)
    where
        K: Into<String>,
        String: PartialEq<K>,
    {
        if let Some(position) = self.write.iter().position(|k| k == key) {
            self.write.remove(position);
        }
    }
}

/// Creates a new role.
pub fn create_role<C>(
    client: &Client<C>,
    role: Role,
) -> Box<Future<Item = Response<Role>, Error = Vec<Error>>>
where
    C: Clone + Connect,
{
    let http_client = client.http_client().clone();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let body = serde_json::to_string(&role)
            .map_err(Error::from)
            .into_future();

        let url = build_url(member, &format!("/roles/{}", role.name));
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let params = uri.join(body);

        let http_client = http_client.clone();

        let response = params.and_then(move |(uri, body)| {
            http_client.put(uri, body).map_err(Error::from)
        });

        let result = response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());
            let body = response.body().concat2().map_err(Error::from);

            body.and_then(move |ref body| if status == StatusCode::Ok {
                match serde_json::from_slice::<Role>(body) {
                    Ok(data) => Ok(Response { data, cluster_info }),
                    Err(error) => Err(Error::Serialization(error)),
                }
            } else {
                Err(Error::UnexpectedStatus(status))
            })
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Creates a new user.
pub fn create_user<C>(
    client: &Client<C>,
    user: NewUser,
) -> Box<Future<Item = Response<User>, Error = Vec<Error>>>
where
    C: Clone + Connect,
{
    let http_client = client.http_client().clone();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let body = serde_json::to_string(&user)
            .map_err(Error::from)
            .into_future();

        let url = build_url(member, &format!("/users/{}", user.name));
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let params = uri.join(body);

        let http_client = http_client.clone();

        let response = params.and_then(move |(uri, body)| {
            http_client.put(uri, body).map_err(Error::from)
        });

        let result = response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());
            let body = response.body().concat2().map_err(Error::from);

            body.and_then(move |ref body| if status == StatusCode::Ok {
                match serde_json::from_slice::<User>(body) {
                    Ok(data) => Ok(Response { data, cluster_info }),
                    Err(error) => Err(Error::Serialization(error)),
                }
            } else {
                Err(Error::UnexpectedStatus(status))
            })
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Deletes a role.
pub fn delete_role<C, N>(
    client: &Client<C>,
    name: N,
) -> Box<Future<Item = Response<()>, Error = Vec<Error>>>
where
    C: Clone + Connect,
    N: Into<String>,
{
    let http_client = client.http_client().clone();
    let name = name.into();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let url = build_url(member, &format!("/roles/{}", name));
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| http_client.delete(uri).map_err(Error::from));

        let result = response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());

            if status == StatusCode::Ok {
                Ok(Response {
                    data: (),
                    cluster_info,
                })
            } else {
                Err(Error::UnexpectedStatus(status))
            }
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Deletes a user.
pub fn delete_user<C, N>(
    client: &Client<C>,
    name: N,
) -> Box<Future<Item = Response<()>, Error = Vec<Error>>>
where
    C: Clone + Connect,
    N: Into<String>,
{
    let http_client = client.http_client().clone();
    let name = name.into();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let url = build_url(member, &format!("/users/{}", name));
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| http_client.delete(uri).map_err(Error::from));

        let result = response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());

            if status == StatusCode::Ok {
                Ok(Response {
                    data: (),
                    cluster_info,
                })
            } else {
                Err(Error::UnexpectedStatus(status))
            }
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Attempts to disable the auth system.
pub fn disable<C>(
    client: &Client<C>,
) -> Box<Future<Item = Response<AuthChange>, Error = Vec<Error>>>
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

            match status {
                StatusCode::Ok => Ok(Response {
                    data: AuthChange::Changed,
                    cluster_info,
                }),
                StatusCode::Conflict => Ok(Response {
                    data: AuthChange::Unchanged,
                    cluster_info,
                }),
                _ => Err(Error::UnexpectedStatus(status)),
            }
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Attempts to enable the auth system.
pub fn enable<C>(client: &Client<C>) -> Box<Future<Item = Response<AuthChange>, Error = Vec<Error>>>
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

        let response = uri.and_then(move |uri| {
            http_client.put(uri, "".to_owned()).map_err(Error::from)
        });

        let result = response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());

            match status {
                StatusCode::Ok => Ok(Response {
                    data: AuthChange::Changed,
                    cluster_info,
                }),
                StatusCode::Conflict => Ok(Response {
                    data: AuthChange::Unchanged,
                    cluster_info,
                }),
                _ => return Err(Error::UnexpectedStatus(status)),
            }
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Get a role.
pub fn get_role<C, N>(
    client: &Client<C>,
    name: N,
) -> Box<Future<Item = Response<Role>, Error = Vec<Error>>>
where
    C: Clone + Connect,
    N: Into<String>,
{
    let http_client = client.http_client().clone();
    let name = name.into();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let url = build_url(member, &format!("/roles/{}", name));
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
                match serde_json::from_slice::<Role>(body) {
                    Ok(data) => Ok(Response { data, cluster_info }),
                    Err(error) => Err(Error::Serialization(error)),
                }
            } else {
                Err(Error::UnexpectedStatus(status))
            })
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Gets all roles.
pub fn get_roles<C>(
    client: &Client<C>,
) -> Box<Future<Item = Response<Vec<Role>>, Error = Vec<Error>>>
where
    C: Clone + Connect,
{
    let http_client = client.http_client().clone();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let url = build_url(member, "/roles");
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
                match serde_json::from_slice::<Roles>(body) {
                    Ok(roles) => {
                        let data = roles.roles.unwrap_or_else(|| Vec::with_capacity(0));

                        Ok(Response { data, cluster_info })
                    }
                    Err(error) => Err(Error::Serialization(error)),
                }
            } else {
                Err(Error::UnexpectedStatus(status))
            })
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Get a user.
pub fn get_user<C, N>(
    client: &Client<C>,
    name: N,
) -> Box<Future<Item = Response<User>, Error = Vec<Error>>>
where
    C: Clone + Connect,
    N: Into<String>,
{
    let http_client = client.http_client().clone();
    let name = name.into();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let url = build_url(member, &format!("/users/{}", name));
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
                match serde_json::from_slice::<User>(body) {
                    Ok(data) => Ok(Response { data, cluster_info }),
                    Err(error) => Err(Error::Serialization(error)),
                }
            } else {
                Err(Error::UnexpectedStatus(status))
            })
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Gets all users.
pub fn get_users<C>(
    client: &Client<C>,
) -> Box<Future<Item = Response<Vec<User>>, Error = Vec<Error>>>
where
    C: Clone + Connect,
{
    let http_client = client.http_client().clone();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let url = build_url(member, "/users");
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
                match serde_json::from_slice::<Users>(body) {
                    Ok(users) => {
                        let data = users.users.unwrap_or_else(|| Vec::with_capacity(0));

                        Ok(Response { data, cluster_info })
                    }
                    Err(error) => Err(Error::Serialization(error)),
                }
            } else {
                Err(Error::UnexpectedStatus(status))
            })
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Determines whether or not the auth system is enabled.
pub fn status<C>(client: &Client<C>) -> Box<Future<Item = Response<bool>, Error = Vec<Error>>>
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

/// Updates an existing role.
pub fn update_role<C>(
    client: &Client<C>,
    role: RoleUpdate,
) -> Box<Future<Item = Response<Role>, Error = Vec<Error>>>
where
    C: Clone + Connect,
{
    let http_client = client.http_client().clone();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let body = serde_json::to_string(&role)
            .map_err(Error::from)
            .into_future();

        let url = build_url(member, &format!("/roles/{}", role.name));
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let params = uri.join(body);

        let http_client = http_client.clone();

        let response = params.and_then(move |(uri, body)| {
            http_client.put(uri, body).map_err(Error::from)
        });

        let result = response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());
            let body = response.body().concat2().map_err(Error::from);

            body.and_then(move |ref body| if status == StatusCode::Ok {
                match serde_json::from_slice::<Role>(body) {
                    Ok(data) => Ok(Response { data, cluster_info }),
                    Err(error) => Err(Error::Serialization(error)),
                }
            } else {
                Err(Error::UnexpectedStatus(status))
            })
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Updates an existing user.
pub fn update_user<C>(
    client: &Client<C>,
    user: UserUpdate,
) -> Box<Future<Item = Response<User>, Error = Vec<Error>>>
where
    C: Clone + Connect,
{
    let http_client = client.http_client().clone();

    let result = first_ok(client.endpoints().to_vec(), move |member| {
        let body = serde_json::to_string(&user)
            .map_err(Error::from)
            .into_future();

        let url = build_url(member, &format!("/users/{}", user.name));
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let params = uri.join(body);

        let http_client = http_client.clone();

        let response = params.and_then(move |(uri, body)| {
            http_client.put(uri, body).map_err(Error::from)
        });

        let result = response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());
            let body = response.body().concat2().map_err(Error::from);

            body.and_then(move |ref body| if status == StatusCode::Ok {
                match serde_json::from_slice::<User>(body) {
                    Ok(data) => Ok(Response { data, cluster_info }),
                    Err(error) => Err(Error::Serialization(error)),
                }
            } else {
                Err(Error::UnexpectedStatus(status))
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
