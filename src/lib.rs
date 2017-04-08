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
//! use std::fs::File;
//! use std::io::Read;
//!
//! use etcd::{Client, ClientOptions, Pkcs12};
//!
//! fn main() {
//!     let mut file = File::open("client.p12").unwrap();
//!     let mut buffer = Vec::new();
//!     file.read_to_end(&mut buffer).unwrap();
//!
//!     let client = Client::with_options(&["https://example.com:2379"], ClientOptions {
//!         pkcs12: Some(Pkcs12::from_der(&buffer, "secret").unwrap()),
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

#![deny(missing_docs)]
#![deny(warnings)]

extern crate hyper;
extern crate hyper_native_tls;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate url;

pub use client::{Client, ClientOptions, Pkcs12};
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
