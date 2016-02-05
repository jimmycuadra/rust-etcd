//! Types for the primary key space operations.

#[cfg(feature = "serde_macros")]
include!("keys_gen.rs");

#[cfg(not(feature = "serde_macros"))]
include!(concat!(env!("OUT_DIR"), "/keys.rs"));
