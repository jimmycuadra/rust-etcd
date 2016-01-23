//! Crate etcd provides a client for [etcd](https://github.com/coreos/etcd), a distributed
//! key-value store from [CoreOS](https://coreos.com/).
//!
//! `Client` is the entry point for all API calls. Types for primary key space operations are
//! reexported to the crate root. Other categories of operations have public types in their
//! respective modules.

extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate url;

pub use client::Client;
pub use error::Error;
pub use keys::{KeySpaceInfo, KeySpaceResult};

pub mod client;
pub mod error;
pub mod keys;
pub mod stats;
pub mod version;

mod http;
mod options;
mod query_pairs;
