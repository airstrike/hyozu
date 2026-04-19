//! Typed errors for the data-loading pipeline.
//!
//! Wraps IO, parse, async-join, schema-build, and cube-construction failures
//! into a single `Error` surfaced to the `Failed` state of the application.

/// Error returned by the async asset loaders and cube construction.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Reading an asset file failed.
    #[error("io at {path}: {source}")]
    Io {
        /// Resolved absolute path of the asset.
        path: String,
        /// Underlying IO error.
        #[source]
        source: std::io::Error,
    },

    /// Parsing an asset's bytes failed.
    #[error("parse {asset}: {detail}")]
    Parse {
        /// Logical asset name (e.g. `"hewton.csv"`).
        asset: &'static str,
        /// Human-readable parse detail from the underlying parser.
        detail: String,
    },

    /// A `tokio::task::spawn_blocking` join failed.
    #[error("join: {0}")]
    Join(#[from] tokio::task::JoinError),

    /// Building the `tatami::Schema` failed.
    #[error("schema: {0}")]
    Schema(#[from] tatami::schema::Error),

    /// Constructing the `tatami_inmem::InMemoryCube` failed.
    #[error("cube: {0}")]
    Cube(#[from] tatami_inmem::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_variant_formats_with_path_and_source() {
        let err = Error::Io {
            path: "/missing.csv".into(),
            source: std::io::Error::other("nope"),
        };
        let msg = format!("{err}");
        assert!(msg.contains("/missing.csv"));
        assert!(msg.contains("nope"));
    }
}
