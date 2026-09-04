//! Parsing and resolution for every language Graphyn understands.
//!
//! Consolidates the five `graphyn-adapter-*` crates and `graphyn-adapter-
//! dispatch` into one crate with a module and a Cargo feature per language.
//! See [`lang`] for why.
//!
//! The entry point is [`analyze_files`], which routes files to the module that
//! owns their language and merges the results into one `RepoIR`.

pub mod dispatch;
pub mod lang;
pub mod spec;
pub mod structural;

pub use dispatch::{analyze_files, supported_languages, DispatchError};
pub use spec::{LanguageSpec, LanguageSupport, Tier};
