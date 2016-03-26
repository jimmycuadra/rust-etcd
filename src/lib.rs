//! Crate etcd provides a client for [etcd](https://github.com/coreos/etcd), a distributed
//! key-value store from [CoreOS](https://coreos.com/).
//!
//! `Client` is the entry point for all API calls.

#![cfg_attr(feature = "serde_macros", feature(custom_derive, plugin))]
#![cfg_attr(feature = "serde_macros", plugin(serde_macros))]

extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate url;

pub use client::Client;
pub use error::{ApiError, EtcdResult, Error};
pub use keys::{KeySpaceInfo, KeySpaceResult, Node};
pub use stats::{
    CountStats,
    FollowerStats,
    LatencyStats,
    LeaderInfo,
    LeaderStats,
    SelfStats,
    StoreStats,
};
pub use version::VersionInfo;

mod client;
mod error;
mod http;
mod keys;
mod options;
mod query_pairs;
mod stats;
mod version;
