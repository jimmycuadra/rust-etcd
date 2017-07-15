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
//! extern crate futures;
//! extern crate tokio_core;
//!
//! use etcd::{Client, kv};
//! use futures::Future;
//! use tokio_core::reactor::Core;
//!
//! fn main() {
//!     let mut core = Core::new().unwrap();
//!     let handle = core.handle();
//!     let client = Client::new(&handle, &["http://etcd.example.com:2379"], None).unwrap();
//!     let work = Box::new(kv::set(&client, "/foo", "bar", None).and_then(|_| {
//!         Box::new(kv::get(&client, "/foo", false, false, false).and_then(|key_space_info| {
//!             let value = key_space_info.node.unwrap().value.unwrap();
//!
//!             assert_eq!(value, "bar".to_string());
//!
//!             Ok(())
//!         }))
//!     }));
//!
//!     core.run(work).unwrap();
//! }
//! ```
//!
//! Using client certificate authentication:
//!
//! ```no_run
//! extern crate etcd;
//! extern crate futures;
//! extern crate hyper;
//! extern crate hyper_tls;
//! extern crate native_tls;
//! extern crate tokio_core;
//!
//! use std::fs::File;
//! use std::io::Read;
//!
//! use futures::Future;
//! use hyper::client::HttpConnector;
//! use hyper_tls::HttpsConnector;
//! use native_tls::{Certificate, Pkcs12, TlsConnector};
//! use tokio_core::reactor::Core;
//!
//! use etcd::{Client, kv};
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
//!     let mut core = Core::new().unwrap();
//!     let handle = core.handle();
//!
//!     let mut http_connector = HttpConnector::new(4, &handle);
//!     http_connector.enforce_http(false);
//!     let https_connector = HttpsConnector::from((http_connector, tls_connector));
//!
//!     let hyper = hyper::Client::configure().connector(https_connector).build(&handle);
//!
//!     let client = Client::custom(hyper, &["https://etcd.example.com:2379"], None).unwrap();
//!
//!     let work = Box::new(kv::set(&client, "/foo", "bar", None).and_then(|_| {
//!         Box::new(kv::get(&client, "/foo", false, false, false).and_then(|key_space_info| {
//!             let value = key_space_info.node.unwrap().value.unwrap();
//!
//!             assert_eq!(value, "bar".to_string());
//!
//!             Ok(())
//!         }))
//!     }));
//!
//!     core.run(work).unwrap();
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
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;
extern crate tokio_timer;
extern crate url;

pub use client::{BasicAuth, Client};
pub use error::{ApiError, Error};
pub use version::VersionInfo;

pub mod kv;
pub mod stats;

mod async;
mod client;
mod error;
mod http;
mod member;
mod options;
mod version;
