//! Types for the primary key space operations.

#[cfg(feature = "serde_derive")]
include!("keys_gen.rs");

#[cfg(not(feature = "serde_derive"))]
include!(concat!(env!("OUT_DIR"), "/keys.rs"));
