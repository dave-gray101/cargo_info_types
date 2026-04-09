//! Parse the output of [`cargo info`] into well-typed Rust structures.
//!
//! ## Overview
//!
//! `cargo_info_types` provides:
//!
//! - [`CrateInfo`] — the top-level structure representing everything `cargo info`
//!   reports about a crate.
//! - [`Feature`] — a single Cargo feature flag with its dependency list.
//! - [`parse()`] — parses the text output of `cargo info -q <crate> --color never`
//!   into a [`CrateInfo`].
//! - [`strip_ansi_escapes()`] — removes ANSI/VT100 color sequences from a string
//!   before passing it to [`parse()`], for cases where `--color never` cannot be
//!   used.
//! - `execute()` *(feature: `execute`)* — runs `cargo info` for a given crate
//!   name and returns the parsed result in one call.
//!
//! ## Quick start
//!
//! Parse a string you already have:
//!
//! ```
//! use cargo_info_types::parse;
//!
//! let output = "syn #macros #syn
//! Parser for Rust source code
//! version: 2.0.117
//! license: MIT OR Apache-2.0
//! rust-version: 1.71
//! documentation: https://docs.rs/syn
//! repository: https://github.com/dtolnay/syn
//! crates.io: https://crates.io/crates/syn/2.0.117
//! features:
//!  +default      = [derive, parsing]
//!   derive       = []
//!   parsing      = []
//! ";
//!
//! let info = parse(output).unwrap();
//! assert_eq!(info.name, "syn");
//! assert_eq!(info.version, "2.0.117");
//! ```
//!
//! Or, with the `execute` feature, run the command directly:
//!
//! ```no_run
//! # #[cfg(feature = "execute")]
//! use cargo_info_types::execute;
//!
//! # #[cfg(feature = "execute")]
//! let info = execute("syn").unwrap();
//! ```
//!
//! ## Feature flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `execute` | Enables command execution, which spawns `cargo info` as a subprocess. Adds no extra dependencies. |
//!
//! [`cargo info`]: https://doc.rust-lang.org/cargo/commands/cargo-info.html

mod executor;
mod parser;
mod types;

pub use parser::{parse, parse_header, parse_feature_line, strip_key, is_known_key_line, RawFields, ParseError};
pub use types::{CrateInfo, Feature};

#[cfg(feature = "execute")]
pub use executor::{execute, ExecuteError};

/// Strips ANSI/VT100 escape sequences from the given string.
///
/// `cargo info` emits ANSI SGR color codes when writing to a terminal. Passing
/// `--color never` to the command suppresses them, but this function provides
/// an alternative when controlling the command flags is not possible, or when
/// you want to pre-process colorized output before passing it to [`parse()`].
///
/// Sequence removal is delegated to the `strip-ansi-escapes` crate, which is
/// backed by the VT100 parser used by terminals such as Alacritty.
///
/// # Examples
///
/// ```
/// use cargo_info_types::strip_ansi_escapes;
///
/// let colored = "\x1b[1;32m+default\x1b[0m      = [std]";
/// assert_eq!(strip_ansi_escapes(colored), "+default      = [std]");
///
/// assert_eq!(strip_ansi_escapes("no escapes here"), "no escapes here");
/// ```
pub fn strip_ansi_escapes(input: &str) -> String {
    // strip_ansi_escapes::strip only removes bytes, so the result is valid
    // UTF-8 whenever the input is.
    String::from_utf8(strip_ansi_escapes::strip(input))
        .expect("stripped output is valid UTF-8")
}


// ── Tests ─────────────────────────────────────────────────────────────────────
// Tests have been moved to: tests/lib.rs
