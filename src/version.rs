//! Types for the version endpoint.

#[cfg(feature = "serde_derive")]
include!("version_gen.rs");

#[cfg(not(feature = "serde_derive"))]
include!(concat!(env!("OUT_DIR"), "/version.rs"));
