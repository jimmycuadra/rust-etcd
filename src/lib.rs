//! Crate etcd provides a client for the [etcd](https://github.com/coreos/etcd) API.
//!
//! All of the public types are rexported and available directly from the crate root. `Client` is
//! the entry point for all API calls.
extern crate hyper;
extern crate rustc_serialize;
extern crate url;

pub use client::Client;
pub use error::Error;
pub use response::{Response, EtcdResult, LeaderStats, FollowerStats, CountStats, LatencyStats};

mod client;
mod error;
mod http;
mod options;
mod query_pairs;
mod response;
