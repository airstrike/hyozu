//! Data layer — schema declaration, load errors, asset parsing.
//!
//! Phase 1 ships the schema (a structure-for-structure copy of the hewton
//! example in `~/projects/tatami/examples/hewton/`) and a typed
//! [`error::Error`] covering IO, parse, and cube-construction failure modes.

pub mod error;
pub mod schema;

pub use error::Error;
pub use schema::hewton_schema;
