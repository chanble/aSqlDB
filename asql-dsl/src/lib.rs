/// SQL dialect traits and built-in dialect implementations
pub mod dialect;
/// Data Definition Language statement builders
pub mod ddl;
/// Data Manipulation Language statement builders
pub mod dml;
/// Data Query Language statement builders
pub mod dql;
/// Database introspection query builders (columns, tables, indexes, etc)
pub mod introspection;

#[cfg(feature = "wasm")]
pub mod wasm;

/// Re-export dialect types for convenient access
pub use dialect::{Dialect, MySql, PostgreSql, Sqlite};
pub use ddl::*;
pub use dml::*;
pub use dql::*;
pub use introspection::*;
