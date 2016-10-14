//! Contains etcd error types.

#[cfg(feature = "serde_derive")]
include!("error_gen.rs");

#[cfg(not(feature = "serde_derive"))]
include!(concat!(env!("OUT_DIR"), "/error.rs"));
