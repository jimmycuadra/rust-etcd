//! Contains etcd error types.

#[cfg(feature = "serde_macros")]
include!("error_gen.rs");

#[cfg(not(feature = "serde_macros"))]
include!(concat!(env!("OUT_DIR"), "/error.rs"));
