//! Crate etcd provides a client for [etcd](https://github.com/coreos/etcd), a distributed
//! key-value store from [CoreOS](https://coreos.com/).
//!
//! `Client` is the entry point for all API calls. Types for etcd's primary key space operations
//! are reexported to the crate root. Types for other etcd operations are found in the crate's
//! public modules.

#![cfg_attr(feature = "serde_macros", feature(custom_derive, plugin))]
#![cfg_attr(feature = "serde_macros", plugin(serde_macros))]

extern crate hyper;
extern crate openssl;
extern crate serde;
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
