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
//! extern crate native_tls;
//!
//! use std::fs::File;
//! use std::io::Read;
//!
//! use native_tls::{Certificate, Pkcs12, TlsConnector};
//!
//! use etcd::{Client, ClientOptions};
//!
//! fn main() {
//!     let mut ca_cert_file = File::open("ca.der").unwrap();
//!     let mut ca_cert_buffer = Vec::new();
//!     ca_cert_file.read_to_end(&mut ca_cert_buffer).unwrap();
//!
//!     let mut pkcs12_file = File::open("/source/tests/ssl/client.p12").unwrap();
//!     let mut pkcs12_buffer = Vec::new();
//!     pkcs12_file.read_to_end(&mut pkcs12_buffer).unwrap();
//!
//!     let mut builder = TlsConnector::builder().unwrap();
//!     builder.add_root_certificate(Certificate::from_der(&ca_cert_buffer).unwrap()).unwrap();
//!     builder.identity(Pkcs12::from_der(&pkcs12_buffer, "secret").unwrap()).unwrap();
//!
//!     let tls_connector = builder.build().unwrap();
//!
//!     let client = Client::with_options(&["https://example.com:2379"], ClientOptions {
//!         tls_connector: Some(tls_connector),
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

#![deny(missing_debug_implementations, missing_docs, warnings)]

extern crate futures;
extern crate hyper;
#[cfg(feature = "tls")]
extern crate hyper_tls;
#[cfg(feature = "tls")]
extern crate native_tls;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;
extern crate url;

pub use client::{BasicAuth, Client};
pub use error::{ApiError, Error};
pub use keys::{KeySpaceInfo, FutureKeySpaceInfo, Node};

pub mod stats;
pub mod version;

mod async;
mod client;
mod error;
mod http;
mod keys;
mod member;
mod options;
