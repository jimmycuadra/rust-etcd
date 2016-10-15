//! Crate etcd provides a client for [etcd](https://github.com/coreos/etcd), a distributed
//! key-value store from [CoreOS](https://coreos.com/).
//!
//! `Client` is the entry point for all API calls. Types for etcd's primary key space operations
//! are reexported to the crate root. Types for other etcd operations are found in the crate's
//! public modules.
//!
//! # Examples
//!
//! Basic usage:
//!
//! ```no_run
//! extern crate etcd;
//!
//! use std::default::Default;
//!
//! use etcd::Client;
//!
//! fn main() {
//!     // Creates a client that will make API calls to http://127.0.0.1:2379.
//!     let client = Client::default();
//!
//!     assert!(client.set("/foo", "bar", None).is_ok());
//!
//!     let value = client.get("/foo", false, false, false)
//!                       .ok().unwrap().node.unwrap().value.unwrap();
//!
//!     assert_eq!(value, "bar".to_owned());
//! }
//! ```
//!
//! Using HTTPS with client certificate authentication and a username and password for HTTP Basic
//! Authorization:
//!
//! ```no_run
//! extern crate etcd;
//!
//! use etcd::{Client, ClientOptions};
//!
//! fn main() {
//!     let client = Client::with_options(&["https://example.com:2379"], ClientOptions {
//!         ca: Some("/path/to/ca_cert.pem".to_owned()),
//!         cert_and_key: Some((
//!             "/path/to/client_cert.pem".to_owned(),
//!             "/path/to/client_key.pem".to_owned(),
//!         )),
//!         username_and_password: Some(("jimmy".to_owned(), "secret".to_owned())),
//!     }).unwrap();
//!
//!     assert!(client.set("/foo", "bar", None).is_ok());
//!
//!     let value = client.get("/foo", false, false, false)
//!                       .ok().unwrap().node.unwrap().value.unwrap();
//!
//!     assert_eq!(value, "bar".to_owned());
//! }
//! ```
#![cfg_attr(feature = "serde_derive", feature(proc_macro))]
#![deny(missing_docs)]

extern crate hyper;
extern crate openssl;
extern crate serde;
#[cfg(feature = "serde_derive")]
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate url;

pub use client::{Client, ClientOptions};
pub use error::{ApiError, EtcdResult, Error};
pub use keys::{KeySpaceInfo, KeySpaceResult, Node};

pub mod stats;
pub mod version;

mod client;
mod error;
mod http;
mod keys;
mod member;
mod options;
mod query_pairs;
