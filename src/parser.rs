use crate::types::{CrateInfo, Feature};
use thiserror::Error;

/// An error that occurred while parsing the output of `cargo info`.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ParseError {
    /// The input string was empty.
    #[error("input is empty")]
    Empty,

    /// The first line (crate name and optional keywords) could not be parsed.
    #[error("invalid header line: {0:?}")]
    InvalidHeader(String),

    /// A required field was absent from the output.
    ///
    /// The contained string names the missing field (e.g., `"version"` or
    /// `"license"`).
    #[error("missing required field: `{0}`")]
    MissingField(&'static str),
}

/// Parses the text output produced by `cargo info -q <crate-name> --color never`.
///
/// The expected format is:
///
/// ```text
/// <name> [#<keyword> ...]
/// <description line(s)>
/// version: <version>
/// license: <license>
/// [rust-version: <msrv>]
/// [documentation: <url>]
/// [homepage: <url>]
/// [repository: <url>]
/// [crates.io: <url>]
/// [features:
///  +<name> = [<dep>, ...]
///   <name> = []]
/// ```
///
/// If the raw output may contain ANSI color sequences (i.e., `--color never`
/// was not passed), call [`crate::strip_ansi_escapes`] on the string first.
///
/// # Errors
///
/// Returns a [`ParseError`] if the input is empty, the header line is
/// malformed, or a required field (`version` or `license`) is absent.
///
/// # Examples
///
/// ```
/// use cargo_info_types::parse;
///
/// let output = "syn #macros #syn
/// Parser for Rust source code
/// version: 2.0.117
/// license: MIT OR Apache-2.0
/// rust-version: 1.71
/// documentation: https://docs.rs/syn
/// repository: https://github.com/dtolnay/syn
/// crates.io: https://crates.io/crates/syn/2.0.117
/// features:
///  +default      = [derive, parsing]
///   derive       = []
///   parsing      = []
/// ";
///
/// let info = parse(output).unwrap();
/// assert_eq!(info.name, "syn");
/// assert_eq!(info.version, "2.0.117");
/// assert_eq!(info.keywords, vec!["macros", "syn"]);
/// assert_eq!(info.features[0].name, "default");
/// assert!(info.features[0].is_default);
/// ```
pub fn parse(input: &str) -> Result<CrateInfo, ParseError> {
    let mut lines = input.lines();

    // ── Phase 1: header ──────────────────────────────────────────────────────
    let header_line = lines.next().ok_or(ParseError::Empty)?;
    let (name, keywords) = parse_header(header_line)?;

    // ── Phases 2–4: description → key-value fields → feature flags ───────────
    let mut description_parts: Vec<&str> = Vec::new();
    let mut fields = RawFields::default();
    let mut features: Vec<Feature> = Vec::new();

    #[derive(PartialEq)]
    enum State {
        Description,
        Fields,
        Features,
    }
    let mut state = State::Description;

    for line in lines {
        match state {
            State::Description => {
                if is_known_key_line(line) {
                    state = if line.starts_with("features:") {
                        State::Features
                    } else {
                        fields.apply(line);
                        State::Fields
                    };
                } else {
                    description_parts.push(line);
                }
            }
            State::Fields => {
                if line.starts_with("features:") {
                    state = State::Features;
                } else {
                    fields.apply(line);
                }
            }
            State::Features => {
                if let Some(feature) = parse_feature_line(line) {
                    features.push(feature);
                }
                // Non-matching lines inside the features block are silently
                // ignored; this allows future additions to the output format
                // without breaking the parser.
            }
        }
    }

    let description = description_parts.join("\n");
    let version = fields.version.ok_or(ParseError::MissingField("version"))?;
    let license = fields.license.ok_or(ParseError::MissingField("license"))?;

    Ok(CrateInfo {
        name,
        keywords,
        description,
        version,
        license,
        rust_version: fields.rust_version,
        documentation: fields.documentation,
        homepage: fields.homepage,
        repository: fields.repository,
        crates_io: fields.crates_io,
        features,
    })
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Parses the first line of `cargo info` output into `(name, keywords)`.
///
/// Input format: `<name>[ #<kw1> #<kw2> ...]`
fn parse_header(line: &str) -> Result<(String, Vec<String>), ParseError> {
    let mut parts = line.split_whitespace();

    let name = parts
        .next()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ParseError::InvalidHeader(line.to_owned()))?;

    let keywords = parts
        .filter_map(|s| s.strip_prefix('#').map(str::to_owned))
        .collect();

    Ok((name, keywords))
}

/// Returns `true` if `line` begins with one of the known field labels produced
/// by `cargo info`.
fn is_known_key_line(line: &str) -> bool {
    const KNOWN_PREFIXES: &[&str] = &[
        "version:",
        "license:",
        "rust-version:",
        "documentation:",
        "homepage:",
        "repository:",
        "crates.io:",
        "features:",
    ];
    KNOWN_PREFIXES.iter().any(|prefix| line.starts_with(prefix))
}

/// Accumulates the optional key-value fields found between the description and
/// the features block.
#[derive(Default)]
struct RawFields {
    version: Option<String>,
    license: Option<String>,
    rust_version: Option<String>,
    documentation: Option<String>,
    homepage: Option<String>,
    repository: Option<String>,
    crates_io: Option<String>,
}

impl RawFields {
    /// Recognizes and stores a single `key: value` line.
    fn apply(&mut self, line: &str) {
        if let Some(v) = strip_key(line, "version:") {
            self.version = Some(v);
        } else if let Some(v) = strip_key(line, "license:") {
            self.license = Some(v);
        } else if let Some(v) = strip_key(line, "rust-version:") {
            self.rust_version = Some(v);
        } else if let Some(v) = strip_key(line, "documentation:") {
            self.documentation = Some(v);
        } else if let Some(v) = strip_key(line, "homepage:") {
            self.homepage = Some(v);
        } else if let Some(v) = strip_key(line, "repository:") {
            self.repository = Some(v);
        } else if let Some(v) = strip_key(line, "crates.io:") {
            self.crates_io = Some(v);
        }
        // "features:" is handled by the caller via state transition.
    }
}

/// Strips `prefix` from the start of `line` and trims the remainder.
/// Returns `None` if `line` does not start with `prefix`.
fn strip_key(line: &str, prefix: &str) -> Option<String> {
    line.strip_prefix(prefix).map(|v| v.trim().to_owned())
}

/// Parses a single feature-flag line from the `features:` block.
///
/// Valid formats:
///
/// ```text
///  +default      = [derive, parsing, printing]
///   clone-impls  = []
/// ```
///
/// Lines that do not match the expected indentation and structure return `None`.
fn parse_feature_line(line: &str) -> Option<Feature> {
    // Feature lines are indented exactly two characters:
    //   ' ' '+' → default (enabled) feature
    //   ' ' ' ' → non-default feature
    let mut chars = line.chars();

    if chars.next() != Some(' ') {
        return None;
    }

    let is_default = match chars.next()? {
        '+' => true,
        ' ' => false,
        _ => return None,
    };

    // Everything from index 2 onward has the form: "name    = [dep, ...]"
    let rest = &line[2..];

    // Locate the " = [" separator.
    let eq_bracket_idx = rest.find(" = [")?;
    let name = rest[..eq_bracket_idx].trim().to_owned();

    if name.is_empty() {
        return None;
    }

    // Extract the content between '[' and the last ']'.
    let after_open = &rest[eq_bracket_idx + 4..]; // skip " = ["
    let close_idx = after_open.rfind(']')?;
    let deps_str = &after_open[..close_idx];

    let dependencies: Vec<String> = if deps_str.trim().is_empty() {
        Vec::new()
    } else {
        deps_str
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect()
    };

    Some(Feature {
        is_default,
        name,
        dependencies,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Real cargo info outputs ───────────────────────────────────────────────

    const SYN_OUTPUT: &str = "\
syn #macros #syn
Parser for Rust source code
version: 2.0.117
license: MIT OR Apache-2.0
rust-version: 1.71
documentation: https://docs.rs/syn
repository: https://github.com/dtolnay/syn
crates.io: https://crates.io/crates/syn/2.0.117
features:
 +default      = [derive, parsing, printing, clone-impls, proc-macro]
  clone-impls  = []
  derive       = []
  parsing      = []
  printing     = [dep:quote]
  proc-macro   = [proc-macro2/proc-macro, quote?/proc-macro]
  extra-traits = []
  fold         = []
  full         = []
  test         = [syn-test-suite/all-features]
  visit        = []
  visit-mut    = []
";

    const SERDE_OUTPUT: &str = "\
serde #serde #serialization #no_std
A generic serialization/deserialization framework
version: 1.0.228
license: MIT OR Apache-2.0
rust-version: 1.56
documentation: https://docs.rs/serde
homepage: https://serde.rs
repository: https://github.com/serde-rs/serde
crates.io: https://crates.io/crates/serde/1.0.228
features:
 +default      = [std]
  std          = [serde_core/std]
  alloc        = [serde_core/alloc]
  derive       = [serde_derive]
  rc           = [serde_core/rc]
  serde_derive = [dep:serde_derive]
  unstable     = [serde_core/unstable]
";

    // tokio has a two-line description
    const TOKIO_OUTPUT: &str = "\
tokio #io #async #non-blocking #futures
An event-driven, non-blocking I/O platform for writing asynchronous I/O
backed applications.
version: 1.51.1
license: MIT
rust-version: 1.71
documentation: https://docs.rs/tokio/1.51.1
homepage: https://tokio.rs
repository: https://github.com/tokio-rs/tokio
crates.io: https://crates.io/crates/tokio/1.51.1
features:
 +default              = []
  fs                   = []
  full                 = [fs, io-util, io-std, macros, net, parking_lot, process, rt, rt-multi-thread, signal, sync, time]
  io-std               = []
  io-util              = [bytes]
  macros               = [tokio-macros]
  rt                   = []
  rt-multi-thread      = [rt]
  sync                 = []
  time                 = []
";

    const ANYHOW_OUTPUT: &str = "\
anyhow #error #error-handling
Flexible concrete Error type built on std::error::Error
version: 1.0.102
license: MIT OR Apache-2.0
rust-version: 1.68
documentation: https://docs.rs/anyhow
repository: https://github.com/dtolnay/anyhow
crates.io: https://crates.io/crates/anyhow/1.0.102
features:
 +default   = [std]
  std       = []
  backtrace = []
";

    // Pre-release version (libc)
    const LIBC_OUTPUT: &str = "\
libc #libc #ffi #bindings #operating #system
Raw FFI bindings to platform libraries like libc.
version: 1.0.0-alpha.3
license: MIT OR Apache-2.0
rust-version: 1.63
documentation: https://docs.rs/libc/1.0.0-alpha.3
repository: https://github.com/rust-lang/libc
crates.io: https://crates.io/crates/libc/1.0.0-alpha.3
features:
 +default                  = [std]
  std                      = []
  extra_traits             = []
  rustc-dep-of-std         = [rustc-std-workspace-core]
  rustc-std-workspace-core = [dep:rustc-std-workspace-core]
";

    // ── parse_header ──────────────────────────────────────────────────────────

    #[test]
    fn header_with_keywords() {
        let (name, kws) = parse_header("syn #macros #syn").unwrap();
        assert_eq!(name, "syn");
        assert_eq!(kws, vec!["macros", "syn"]);
    }

    #[test]
    fn header_without_keywords() {
        let (name, kws) = parse_header("my-crate").unwrap();
        assert_eq!(name, "my-crate");
        assert!(kws.is_empty());
    }

    #[test]
    fn header_many_keywords() {
        let (name, kws) = parse_header("tokio #io #async #non-blocking #futures").unwrap();
        assert_eq!(name, "tokio");
        assert_eq!(kws, vec!["io", "async", "non-blocking", "futures"]);
    }

    #[test]
    fn header_empty_returns_error() {
        assert_eq!(
            parse_header(""),
            Err(ParseError::InvalidHeader("".to_owned()))
        );
    }

    // ── parse_feature_line ────────────────────────────────────────────────────

    #[test]
    fn feature_line_default_no_deps() {
        let f = parse_feature_line(" +default   = []").unwrap();
        assert_eq!(f.name, "default");
        assert!(f.is_default);
        assert!(f.dependencies.is_empty());
    }

    #[test]
    fn feature_line_non_default_no_deps() {
        let f = parse_feature_line("  derive       = []").unwrap();
        assert_eq!(f.name, "derive");
        assert!(!f.is_default);
        assert!(f.dependencies.is_empty());
    }

    #[test]
    fn feature_line_with_multiple_deps() {
        let line = " +default      = [derive, parsing, printing, clone-impls, proc-macro]";
        let f = parse_feature_line(line).unwrap();
        assert!(f.is_default);
        assert_eq!(f.name, "default");
        assert_eq!(
            f.dependencies,
            vec!["derive", "parsing", "printing", "clone-impls", "proc-macro"]
        );
    }

    #[test]
    fn feature_line_dep_colon_prefix() {
        let f = parse_feature_line("  printing     = [dep:quote]").unwrap();
        assert_eq!(f.dependencies, vec!["dep:quote"]);
    }

    #[test]
    fn feature_line_dep_slash_optional() {
        let f =
            parse_feature_line("  proc-macro   = [proc-macro2/proc-macro, quote?/proc-macro]")
                .unwrap();
        assert_eq!(
            f.dependencies,
            vec!["proc-macro2/proc-macro", "quote?/proc-macro"]
        );
    }

    #[test]
    fn feature_line_dep_optional_dep() {
        let f = parse_feature_line("  rustc-std-workspace-core = [dep:rustc-std-workspace-core]")
            .unwrap();
        assert_eq!(f.dependencies, vec!["dep:rustc-std-workspace-core"]);
    }

    #[test]
    fn strip_key_missing_prefix_returns_none() {
        assert_eq!(strip_key("version 1.0.0", "version:"), None);
    }

    #[test]
    fn raw_fields_apply_recognizes_all_optional_fields() {
        let mut fields = RawFields::default();
        fields.apply("version: 0.2.0");
        fields.apply("license: Apache-2.0");
        fields.apply("rust-version: 1.70");
        fields.apply("documentation: https://docs.rs/example");
        fields.apply("homepage: https://example.com");
        fields.apply("repository: https://example.com/repo");
        fields.apply("crates.io: https://crates.io/crates/example/0.2.0");
        fields.apply("unknown: ignored");

        assert_eq!(fields.version.as_deref(), Some("0.2.0"));
        assert_eq!(fields.license.as_deref(), Some("Apache-2.0"));
        assert_eq!(fields.rust_version.as_deref(), Some("1.70"));
        assert_eq!(fields.documentation.as_deref(), Some("https://docs.rs/example"));
        assert_eq!(fields.homepage.as_deref(), Some("https://example.com"));
        assert_eq!(fields.repository.as_deref(), Some("https://example.com/repo"));
        assert_eq!(fields.crates_io.as_deref(), Some("https://crates.io/crates/example/0.2.0"));
    }

    #[test]
    fn parse_feature_line_invalid_prefix_returns_none() {
        assert!(parse_feature_line("x default = []").is_none());
    }

    #[test]
    fn parse_feature_line_invalid_second_character_returns_none() {
        assert!(parse_feature_line(" @invalid = []").is_none());
    }

    #[test]
    fn parse_feature_line_missing_name_returns_none() {
        assert!(parse_feature_line(" +    = []").is_none());
    }

    #[test]
    fn parse_ignores_invalid_feature_lines_inside_features_block() {
        let input = "my-crate\nA description\nversion: 0.1.0\nlicense: MIT\nfeatures:\n  invalid\n +default = []\n";
        let info = parse(input).unwrap();
        assert_eq!(info.features.len(), 1);
        assert_eq!(info.features[0].name, "default");
    }

    #[test]
    fn is_known_key_line_recognizes_known_prefixes() {
        assert!(is_known_key_line("version: 1.0.0"));
        assert!(is_known_key_line("license: MIT"));
        assert!(is_known_key_line("rust-version: 1.71"));
        assert!(is_known_key_line("features:"));
        assert!(!is_known_key_line("some random text"));
    }

    #[test]
    fn feature_line_non_feature_returns_none() {
        assert!(parse_feature_line("version: 1.0.0").is_none());
        assert!(parse_feature_line("").is_none());
        assert!(parse_feature_line("features:").is_none());
    }

    // ── Full parse: syn ───────────────────────────────────────────────────────

    #[test]
    fn parse_syn_name_and_keywords() {
        let info = parse(SYN_OUTPUT).unwrap();
        assert_eq!(info.name, "syn");
        assert_eq!(info.keywords, vec!["macros", "syn"]);
    }

    #[test]
    fn parse_syn_description() {
        let info = parse(SYN_OUTPUT).unwrap();
        assert_eq!(info.description, "Parser for Rust source code");
    }

    #[test]
    fn parse_syn_version_and_license() {
        let info = parse(SYN_OUTPUT).unwrap();
        assert_eq!(info.version, "2.0.117");
        assert_eq!(info.license, "MIT OR Apache-2.0");
    }

    #[test]
    fn parse_syn_optional_fields() {
        let info = parse(SYN_OUTPUT).unwrap();
        assert_eq!(info.rust_version.as_deref(), Some("1.71"));
        assert_eq!(info.documentation.as_deref(), Some("https://docs.rs/syn"));
        assert!(info.homepage.is_none());
        assert_eq!(
            info.repository.as_deref(),
            Some("https://github.com/dtolnay/syn")
        );
        assert_eq!(
            info.crates_io.as_deref(),
            Some("https://crates.io/crates/syn/2.0.117")
        );
    }

    #[test]
    fn parse_syn_features_count() {
        let info = parse(SYN_OUTPUT).unwrap();
        assert_eq!(info.features.len(), 12);
    }

    #[test]
    fn parse_syn_default_feature() {
        let info = parse(SYN_OUTPUT).unwrap();
        let default_feat = &info.features[0];
        assert_eq!(default_feat.name, "default");
        assert!(default_feat.is_default);
        assert_eq!(
            default_feat.dependencies,
            vec!["derive", "parsing", "printing", "clone-impls", "proc-macro"]
        );
    }

    #[test]
    fn parse_syn_non_default_feature() {
        let info = parse(SYN_OUTPUT).unwrap();
        let derive_feat = info.features.iter().find(|f| f.name == "derive").unwrap();
        assert!(!derive_feat.is_default);
        assert!(derive_feat.dependencies.is_empty());
    }

    #[test]
    fn parse_syn_feature_with_dep_colon() {
        let info = parse(SYN_OUTPUT).unwrap();
        let printing = info
            .features
            .iter()
            .find(|f| f.name == "printing")
            .unwrap();
        assert_eq!(printing.dependencies, vec!["dep:quote"]);
    }

    #[test]
    fn parse_syn_feature_complex_deps() {
        let info = parse(SYN_OUTPUT).unwrap();
        let proc_macro = info
            .features
            .iter()
            .find(|f| f.name == "proc-macro")
            .unwrap();
        assert_eq!(
            proc_macro.dependencies,
            vec!["proc-macro2/proc-macro", "quote?/proc-macro"]
        );
    }

    // ── Full parse: serde (has homepage) ─────────────────────────────────────

    #[test]
    fn parse_serde_homepage_present() {
        let info = parse(SERDE_OUTPUT).unwrap();
        assert_eq!(info.homepage.as_deref(), Some("https://serde.rs"));
    }

    #[test]
    fn parse_serde_keywords() {
        let info = parse(SERDE_OUTPUT).unwrap();
        assert_eq!(info.keywords, vec!["serde", "serialization", "no_std"]);
    }

    #[test]
    fn parse_serde_features() {
        let info = parse(SERDE_OUTPUT).unwrap();
        assert_eq!(info.features.len(), 7);

        let default_feat = &info.features[0];
        assert!(default_feat.is_default);
        assert_eq!(default_feat.dependencies, vec!["std"]);
    }

    // ── Full parse: tokio (multi-line description) ────────────────────────────

    #[test]
    fn parse_tokio_multiline_description() {
        let info = parse(TOKIO_OUTPUT).unwrap();
        assert_eq!(
            info.description,
            "An event-driven, non-blocking I/O platform for writing asynchronous I/O\nbacked applications."
        );
    }

    #[test]
    fn parse_tokio_keywords() {
        let info = parse(TOKIO_OUTPUT).unwrap();
        assert_eq!(info.keywords, vec!["io", "async", "non-blocking", "futures"]);
    }

    #[test]
    fn parse_tokio_default_feature_empty_deps() {
        let info = parse(TOKIO_OUTPUT).unwrap();
        let default_feat = &info.features[0];
        assert!(default_feat.is_default);
        assert!(default_feat.dependencies.is_empty());
    }

    // ── Full parse: libc (pre-release version) ───────────────────────────────

    #[test]
    fn parse_libc_prerelease_version() {
        let info = parse(LIBC_OUTPUT).unwrap();
        assert_eq!(info.version, "1.0.0-alpha.3");
    }

    // ── Full parse: anyhow ────────────────────────────────────────────────────

    #[test]
    fn parse_anyhow_no_homepage() {
        let info = parse(ANYHOW_OUTPUT).unwrap();
        assert!(info.homepage.is_none());
    }

    #[test]
    fn parse_anyhow_features() {
        let info = parse(ANYHOW_OUTPUT).unwrap();
        assert_eq!(info.features.len(), 3);
    }

    // ── Error cases ───────────────────────────────────────────────────────────

    #[test]
    fn error_on_empty_input() {
        assert_eq!(parse(""), Err(ParseError::Empty));
    }

    #[test]
    fn error_on_missing_version() {
        let input = "my-crate\nA description\nlicense: MIT\n";
        assert_eq!(parse(input), Err(ParseError::MissingField("version")));
    }

    #[test]
    fn error_on_missing_license() {
        let input = "my-crate\nA description\nversion: 1.0.0\n";
        assert_eq!(parse(input), Err(ParseError::MissingField("license")));
    }

    // ── Edge cases ────────────────────────────────────────────────────────────

    #[test]
    fn no_features_section() {
        let input = "my-crate\nA simple crate\nversion: 0.1.0\nlicense: MIT\n";
        let info = parse(input).unwrap();
        assert!(info.features.is_empty());
    }

    #[test]
    fn empty_features_section() {
        let input = "my-crate\nA simple crate\nversion: 0.1.0\nlicense: MIT\nfeatures:\n";
        let info = parse(input).unwrap();
        assert!(info.features.is_empty());
    }

    #[test]
    fn no_keywords() {
        let input = "my-crate\nA description\nversion: 1.0.0\nlicense: MIT\n";
        let info = parse(input).unwrap();
        assert_eq!(info.name, "my-crate");
        assert!(info.keywords.is_empty());
    }

    #[test]
    fn all_optional_fields_absent() {
        let input = "mini-crate\nDoes one thing\nversion: 0.1.0\nlicense: MIT\n";
        let info = parse(input).unwrap();
        assert!(info.rust_version.is_none());
        assert!(info.documentation.is_none());
        assert!(info.homepage.is_none());
        assert!(info.repository.is_none());
        assert!(info.crates_io.is_none());
    }

    #[test]
    fn feature_line_dependencies_with_whitespace() {
        // Test that the filter for empty strings after trim works
        let line = "  feature     = [dep1, , dep2]";
        if let Some(f) = parse_feature_line(line) {
            // Whitespace-only deps should be filtered out
            assert!(!f.dependencies.iter().any(|d| d.is_empty()));
        }
    }

    #[test]
    fn feature_line_with_spaces_between_deps() {
        // Ensure the filter removes entries that become empty after trimming
        let line = "  myfeature   = [a,   ,b]";
        if let Some(f) = parse_feature_line(line) {
            // All dependencies should have content
            assert!(f.dependencies.iter().all(|d| !d.is_empty() && !d.trim().is_empty()));
        }
    }

    #[test]
    fn parse_with_all_optional_fields_present() {
        // Comprehensive test covering all optional fields in parse context
        let input = "testcrate #test
A test crate
version: 1.0.0
license: MIT
rust-version: 1.70
documentation: https://docs.rs/test
homepage: https://test.com
repository: https://github.com/test/test
crates.io: https://crates.io/crates/test/1.0.0
features:
 +default = []
";
        let info = parse(input).unwrap();
        assert_eq!(info.name, "testcrate");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.license, "MIT");
        assert_eq!(info.rust_version.as_deref(), Some("1.70"));
        assert_eq!(info.documentation.as_deref(), Some("https://docs.rs/test"));
        assert_eq!(info.homepage.as_deref(), Some("https://test.com"));
        assert_eq!(info.repository.as_deref(), Some("https://github.com/test/test"));
        assert_eq!(info.crates_io.as_deref(), Some("https://crates.io/crates/test/1.0.0"));
    }

    #[test]
    fn feature_line_with_multiple_spaces_in_deps() {
        // Test edge case with multiple consecutive spaces between dependencies
        let line = "  test = [a,     b, c]";
        if let Some(f) = parse_feature_line(line) {
            assert_eq!(f.dependencies, vec!["a", "b", "c"]);
        }
    }
}
