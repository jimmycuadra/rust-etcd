//! Types for statistics operations.

#[cfg(feature = "serde_derive")]
include!("stats_gen.rs");

#[cfg(not(feature = "serde_derive"))]
include!(concat!(env!("OUT_DIR"), "/stats.rs"));
