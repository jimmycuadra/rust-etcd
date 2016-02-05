//! Types for statistics operations.

#[cfg(feature = "serde_macros")]
include!("stats_gen.rs");

#[cfg(not(feature = "serde_macros"))]
include!(concat!(env!("OUT_DIR"), "/stats.rs"));
