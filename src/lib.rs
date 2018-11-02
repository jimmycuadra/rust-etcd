//! Crate `etcd` provides a client for [etcd](https://github.com/coreos/etcd), a distributed
//! key-value store from [CoreOS](https://coreos.com/).
//!
//! The client uses etcd's v2 API. Support for the v3 API is planned, and will be added via
//! separate types for backwards compatibility and to support both APIs simultaneously.
//!
//! The client uses asynchronous I/O, backed by the `futures` and `tokio-core` crates, and requires
//! both to be used alongside. Functions that return futures currently return them boxed. As soon
//! as Rust's [impl Trait](https://github.com/rust-lang/rust/issues/34511) feature is stabilized,
//! this crate's API will be updated to use it.
//!
//! The client is tested against etcd 2.3.7.
//!
//! # Usage
//!
//! `Client` is an HTTP client required for all API calls. It can be constructed to use HTTP or
//! HTTPS, and supports authenticating to the etcd cluster via HTTP basic authentication (username
//! and password) and/or X.509 client certificates.
//!
//! To get basic information about the versions of etcd running in a cluster, use the
//! `Client::versions` method. All other API calls are made by passing a `Client` reference to the
//! functions in the `kv`, `members`, and `stats` modules. These modules contain functions for API
//! calls to the primary key-value store API, the cluster membership API, and statistics API,
//! respectively. The authentication API is not yet supported, but planned.
//!
//! # Examples
//!
//! Basic usage:
//!
//! ```no_run
//! extern crate etcd;
//! extern crate futures;
//! extern crate tokio_core;
//!
//! use etcd::Client;
//! use etcd::kv::{self, Action};
//! use futures::Future;
//! use tokio_core::reactor::Core;
//!
//! fn main() {
//!     // Create a `Core`, which is the event loop which will drive futures to completion.
//!     let mut core = Core::new().unwrap();
//!
//!     // Create a client to access a single cluster member. Addresses of multiple cluster
//!     // members can be provided and the client will try each one in sequence until it
//!     // receives a successful response.
//!     let client = Client::new(&["http://etcd.example.com:2379"], None).unwrap();
//!
//!     // Set the key "/foo" to the value "bar" with no expiration.
//!     let work = kv::set(&client, "/foo", "bar", None).and_then(|_| {
//!         // Once the key has been set, ask for details about it.
//!         let get_request = kv::get(&client, "/foo", kv::GetOptions::default());
//!
//!         get_request.and_then(|response| {
//!             // The information returned tells you what kind of operation was performed.
//!             assert_eq!(response.data.action, Action::Get);
//!
//!             // The value of the key is what we set it to previously.
//!             assert_eq!(response.data.node.value, Some("bar".to_string()));
//!
//!             // Each API call also returns information about the etcd cluster extracted from
//!             // HTTP response headers.
//!             assert!(response.cluster_info.etcd_index.is_some());
//!
//!             Ok(())
//!         })
//!     });
//!
//!     // Start the event loop, driving the asynchronous code to completion.
//!     core.run(work).unwrap();
//! }
//! ```
//!
//! # Cargo features
//!
//! Crate `etcd` has one Cargo feature, `tls`, which adds HTTPS support via the `Client::https`
//! constructor. This feature is enabled by default.
#![deny(missing_debug_implementations, missing_docs, warnings)]

extern crate futures;
// #[macro_use]
extern crate hyper;
extern crate http as hyper_http;
#[cfg(feature = "tls")]
extern crate hyper_tls;
#[cfg(feature = "tls")]
extern crate native_tls;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;
extern crate tokio_timer;
extern crate url;
extern crate base64;
#[macro_use]
extern crate log;

pub use client::{BasicAuth, Client, ClusterInfo, Health, Response};
pub use error::{ApiError, Error};
pub use version::VersionInfo;

pub mod auth;
pub mod kv;
pub mod members;
pub mod stats;

mod async;
mod client;
mod error;
mod http;
mod options;
mod version;
