//! Types for the version endpoint.

#[cfg(feature = "serde_macros")]
include!("version_gen.rs");

#[cfg(not(feature = "serde_macros"))]
include!(concat!(env!("OUT_DIR"), "/version.rs"));
