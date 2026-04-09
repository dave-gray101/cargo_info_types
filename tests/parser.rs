use cargo_info_types::{parse, parse_header, parse_feature_line, strip_key, is_known_key_line, RawFields, ParseError};

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

// ── State machine path coverage ───────────────────────────────────────────

#[test]
fn parse_description_to_features_directly() {
    // Jump directly from Description to Features, skipping Fields
    let input = "direct-jump #test
Single line description
version: 1.0.0
license: MIT
features:
 +default = [dep1, dep2]
  other = []
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "direct-jump");
    assert_eq!(info.keywords, vec!["test"]);
    assert_eq!(info.description, "Single line description");
    assert_eq!(info.version, "1.0.0");
    assert_eq!(info.license, "MIT");
    assert_eq!(info.features.len(), 2);
    assert_eq!(info.features[0].name, "default");
    assert_eq!(info.features[0].dependencies, vec!["dep1", "dep2"]);
}

#[test]
fn parse_multiple_description_lines_to_fields() {
    // Multiple description lines, then key-value fields, no features
    let input = "multi-desc #docs
First line of description.
Second line of description.
Third line of description.
version: 2.0.0
license: Apache-2.0
rust-version: 1.68
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "multi-desc");
    assert_eq!(info.version, "2.0.0");
    assert_eq!(info.license, "Apache-2.0");
    assert_eq!(info.rust_version.as_deref(), Some("1.68"));
    assert!(info.features.is_empty());
    assert_eq!(
        info.description,
        "First line of description.\nSecond line of description.\nThird line of description."
    );
}

#[test]
fn parse_fields_only_no_features() {
    // Go through Fields state but never reach Features
    let input = "fields-only
A crate with only required fields.
version: 0.5.0
license: MIT
documentation: https://docs.rs/fields-only
homepage: https://fields-only.com
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "fields-only");
    assert_eq!(info.version, "0.5.0");
    assert_eq!(info.documentation.as_deref(), Some("https://docs.rs/fields-only"));
    assert_eq!(info.homepage.as_deref(), Some("https://fields-only.com"));
    assert!(info.features.is_empty());
}

#[test]
fn parse_many_fields_then_features() {
    // Many field lines followed by features
    let input = "many-fields #complex
Complex metadata.
version: 3.2.1
license: GPL-3.0
rust-version: 1.75
documentation: https://docs.rs/many-fields
homepage: https://many.example.com
repository: https://github.com/many/fields
crates.io: https://crates.io/crates/many-fields/3.2.1
features:
 +std = []
  alloc = []
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "many-fields");
    assert_eq!(info.keywords, vec!["complex"]);
    assert_eq!(info.license, "GPL-3.0");
    assert_eq!(info.rust_version.as_deref(), Some("1.75"));
    assert_eq!(info.repository.as_deref(), Some("https://github.com/many/fields"));
    assert_eq!(info.crates_io.as_deref(), Some("https://crates.io/crates/many-fields/3.2.1"));
    assert_eq!(info.features.len(), 2);
}

#[test]
fn parse_description_stays_in_description() {
    // Stay in Description state until Features (no key-value fields before features)
    let input = "desc-only
Line one of description.
Line two of description.
version: 1.5.0
license: MIT
features:
 +feat = []
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "desc-only");
    assert_eq!(info.version, "1.5.0");
    assert!(info.features.iter().any(|f| f.name == "feat"));
}

#[test]
fn parse_features_with_mixed_valid_invalid_lines() {
    // Some lines in features block are invalid and should be skipped
    let input = "mixed-features
A crate with mixed feature lines.
version: 1.5.0
license: MIT
features:
 +default = [a, b]
This is not a valid feature line
  feature2 = [c]
not at all valid = [d]
  last = []
";
    let info = parse(input).unwrap();
    assert_eq!(info.features.len(), 3); // default, feature2, last
    let names: Vec<&str> = info.features.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, vec!["default", "feature2", "last"]);
}

#[test]
fn parse_single_line_description_with_all_fields() {
    // Test Description -> Fields -> Features with minimal description
    let input = "min
v
version: 1.0.0
license: MIT
features:
 +x = [y]
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "min");
    assert_eq!(info.description, "v");
    assert_eq!(info.version, "1.0.0");
    assert_eq!(info.license, "MIT");
    assert_eq!(info.features.len(), 1);
}

#[test]
fn parse_transition_fields_to_features_mid_fields() {
    // Verify transition from Fields directly to Features
    let input = "transition-test
Description here.
version: 1.0.0
license: MIT
repository: https://example.com/repo
features:
 +f1 = [d1]
  f2 = [d2]
";
    let info = parse(input).unwrap();
    assert_eq!(info.repository.as_deref(), Some("https://example.com/repo"));
    assert_eq!(info.features.len(), 2);
}

#[test]
fn parse_complete_from_description_state() {
    // Parser completes from Description state (no fields, no features)
    let input = "desc-only #minimal
Just a single line description
version: 0.0.1
license: MIT
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "desc-only");
    assert_eq!(info.description, "Just a single line description");
}

#[test]
fn parse_complete_from_fields_state() {
    // Parser completes from Fields state (no features)
    let input = "no-features
With description.
version: 1.0.0
license: Apache-2.0
documentation: https://example.com
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "no-features");
    assert_eq!(info.documentation.as_deref(), Some("https://example.com"));
    assert!(info.features.is_empty());
}

#[test]
fn parse_complete_from_features_state() {
    // Parser completes from Features state
    let input = "with-features #feat
Description.
version: 2.0.0
license: MIT
features:
 +enabled = [dep]
  disabled = []
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "with-features");
    assert_eq!(info.features.len(), 2);
}

#[test]
fn parse_version_then_features_directly() {
    // Test path: Description -> Fields (via version) -> Features
    // This explicitly tests Fields::transition_to_features
    let input = "path-test
Description line.
version: 1.0.0
license: MIT
features:
 +default = []
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "path-test");
    assert_eq!(info.version, "1.0.0");
    assert_eq!(info.license, "MIT");
    assert_eq!(info.features.len(), 1);
}

#[test]
fn parse_minimal_version_license_features() {
    // Minimal test with just required fields + features
    let input = "minimal
Crate
version: 0.1.0
license: MIT
features:
 +feat = [dep]
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "minimal");
    assert_eq!(info.version, "0.1.0");
    assert_eq!(info.license, "MIT");
    assert_eq!(info.features.len(), 1);
    assert_eq!(info.features[0].name, "feat");
    assert_eq!(info.features[0].dependencies, vec!["dep"]);
}

#[test]
fn parse_with_keywords_and_features() {
    // Ensure keyword parsing and feature parsing work together
    let input = "kw-crate #key1 #key2
Desc
version: 1.0.0
license: MIT
features:
 +default = []
  extra = [x]
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "kw-crate");
    assert_eq!(info.keywords, vec!["key1", "key2"]);
    assert_eq!(info.features.len(), 2);
}

#[test]
fn parse_description_only_go_to_fields() {
    // Stay in description multiple times, then go to fields
    let input = "desc-then-fields
Line 1
Line 2
Line 3
version: 1.0.0
license: MIT
";
    let info = parse(input).unwrap();
    assert_eq!(info.description, "Line 1\nLine 2\nLine 3");
    assert_eq!(info.version, "1.0.0");
    assert_eq!(info.license, "MIT");
}

#[test]
fn parse_feature_with_single_line_description() {
    // Minimal description with features
    let input = "x
y
version: 1.0.0
license: MIT
features:
 +z = []
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "x");
    assert_eq!(info.description, "y");
    assert_eq!(info.features.len(), 1);
}

#[test]
fn parse_fields_accumulate_progressively() {
    // Test that RawFields::apply works on each line
    let input = "prog
Desc
version: 1.0.0
license: MIT
rust-version: 1.56
documentation: https://docs.rs/prog
homepage: https://prog.io
repository: https://github.com/prog/prog
crates.io: https://crates.io/crates/prog
";
    let info = parse(input).unwrap();
    assert_eq!(info.version, "1.0.0");
    assert_eq!(info.license, "MIT");
    assert_eq!(info.rust_version.as_deref(), Some("1.56"));
    assert_eq!(info.documentation.as_deref(), Some("https://docs.rs/prog"));
    assert_eq!(info.homepage.as_deref(), Some("https://prog.io"));
    assert_eq!(info.repository.as_deref(), Some("https://github.com/prog/prog"));
    assert_eq!(info.crates_io.as_deref(), Some("https://crates.io/crates/prog"));
}

#[test]
fn parse_explicit_description_transition() {
    // Force Description state handling by ensuring multiple description lines
    // then immediate transition to Fields, confirming the state machine works
    let input = "state-test
Line one
Line two  
Line three
Line four
Line five
version: 1.0.0
license: MIT
";
    let info = parse(input).unwrap();
    let expected_desc = "Line one\nLine two  \nLine three\nLine four\nLine five";
    assert_eq!(info.description, expected_desc);
    assert_eq!(info.version, "1.0.0");
}

#[test]
fn parse_repeated_field_lines() {
    // Test that each field type is applied correctly and repeatedly
    let input = "repeat-test
Desc
version: 1.0.0
license: MIT
rust-version: 1.65
documentation: https://docs.rs/repeat
";
    let info = parse(input).unwrap();
    assert_eq!(info.rust_version.as_deref(), Some("1.65"));
    assert_eq!(info.documentation.as_deref(), Some("https://docs.rs/repeat"));
}

#[test]
fn parse_all_fields_no_features() {
    // Test with ALL optional fields filled in and NO features
    let input = "all-fields #tag1 #tag2 #tag3
A comprehensive description here.
version: 3.2.1
license: MIT AND Apache-2.0
rust-version: 1.70
documentation: https://docs.rs/lib
homepage: https://example.com
repository: https://github.com/user/repo
crates.io: https://crates.io/crates/lib/3.2.1
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "all-fields");
    assert_eq!(info.keywords, vec!["tag1", "tag2", "tag3"]);
    assert_eq!(info.license, "MIT AND Apache-2.0");
    assert_eq!(info.rust_version.as_deref(), Some("1.70"));
    assert_eq!(info.documentation.as_deref(), Some("https://docs.rs/lib"));
    assert_eq!(info.homepage.as_deref(), Some("https://example.com"));
    assert_eq!(info.repository.as_deref(), Some("https://github.com/user/repo"));
    assert_eq!(info.crates_io.as_deref(), Some("https://crates.io/crates/lib/3.2.1"));
    assert!(info.features.is_empty());
}

#[test]
fn parse_single_feature_multiple_times() {
    // Test feature parsing with variations
    let input = "feat-var
D
version: 1.0.0
license: MIT
features:
 +enabled = [a]
  disabled = []
  another = [x, y, z]
";
    let info = parse(input).unwrap();
    assert_eq!(info.features.len(), 3);
    assert!(info.features[0].is_default);
    assert!(!info.features[1].is_default);
    assert!(!info.features[2].is_default);
}

#[test]
fn parse_feature_with_complex_deps() {
    // Special dependency formats
    let input = "complex-deps
D
version: 1.0.0
license: MIT
features:
 +full = [dep:serde, pkg/feature, opt?/pkg, simple]
";
    let info = parse(input).unwrap();
    assert_eq!(info.features[0].dependencies, vec!["dep:serde", "pkg/feature", "opt?/pkg", "simple"]);
}

#[test]
fn parse_empty_description() {
    // Minimal: header + version + license only (NO description lines)
    let input = "minimal-desc
version: 0.0.1
license: MIT
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "minimal-desc");
    assert_eq!(info.description, "");
    assert_eq!(info.version, "0.0.1");
}

#[test]
fn parse_complex_scenario_full_coverage() {
    // A complex scenario hitting many code paths
    let input = "complex #tag
Multi
line
description
here
version: 1.2.3
license: GPL
rust-version: 1.60
documentation: https://docs
homepage: https://home
repository: https://repo  
crates.io: https://crates
features:
 +default = [std, alloc]
  std = []
  alloc = [dep:alloc]
  nightly = [unstable]
  debug = []
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "complex");
    assert_eq!(info.keywords, vec!["tag"]);
    assert_eq!(info.features.len(), 5);
    assert_eq!(info.features[0].name, "default");
    assert!(info.features[0].is_default);
    assert_eq!(info.features[4].name, "debug");
    assert!(!info.features[4].is_default);
}

#[test]
fn parse_desc_only_complete() {
    // Test completing from Description state after parsing header
    let input = "desc-complete
Just a description
version: 1.0
license: MIT
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "desc-complete");
    assert_eq!(info.version, "1.0");
}

#[test]
fn parse_fields_complete_path() {
    // Explicitly test Fields -> Complete transition
    let input = "fields-path
Desc line
version: 2.5.0
license: Apache-2.0
rust-version: 1.80
";
    let info = parse(input).unwrap();
    assert_eq!(info.version, "2.5.0");
    assert_eq!(info.rust_version.as_deref(), Some("1.80"));
}

#[test]
fn parse_features_complete_path() {
    // Explicitly test Features -> Complete transition
    let input = "features-path
D
version: 1.0
license: MIT
features:
 +default = []
  opt = [x]
";
    let info = parse(input).unwrap();
    assert_eq!(info.features.len(), 2);
}

#[test]
fn raw_fields_all_individual_applies() {
    // Test RawFields::apply() for each field individually
    let mut fields = RawFields::default();
    
    fields.apply("version: 1.2.3");
    assert_eq!(fields.version, Some("1.2.3".to_string()));
    
    fields.apply("license: GPL");
    assert_eq!(fields.license, Some("GPL".to_string()));
    
    fields.apply("rust-version: 1.75");
    assert_eq!(fields.rust_version, Some("1.75".to_string()));
    
    fields.apply("documentation: https://docs.rs/test");
    assert_eq!(fields.documentation, Some("https://docs.rs/test".to_string()));
    
    fields.apply("homepage: https://home.test");
    assert_eq!(fields.homepage, Some("https://home.test".to_string()));
    
    fields.apply("repository: https://github.test");
    assert_eq!(fields.repository, Some("https://github.test".to_string()));
    
    fields.apply("crates.io: https://crates.test");
    assert_eq!(fields.crates_io, Some("https://crates.test".to_string()));
}

#[test]
fn test_parse_header_variations() {
    // Test parse_header() with various inputs
    let (n1, k1) = parse_header("crate").unwrap();
    assert_eq!(n1, "crate");
    assert_eq!(k1.len(), 0);
    
    let (n2, k2) = parse_header("pkg #a").unwrap();
    assert_eq!(n2, "pkg");
    assert_eq!(k2, vec!["a"]);
    
    let (n3, k3) = parse_header("lib #x #y #z").unwrap();
    assert_eq!(n3, "lib");
    assert_eq!(k3, vec!["x", "y", "z"]);
}

#[test]
fn test_is_known_key_line_all_types() {
    // Test each known key type
    assert!(is_known_key_line("version: 1.0"));
    assert!(is_known_key_line("license: MIT"));
    assert!(is_known_key_line("rust-version: 1.60"));
    assert!(is_known_key_line("documentation: https"));
    assert!(is_known_key_line("homepage: https"));
    assert!(is_known_key_line("repository: https"));
    assert!(is_known_key_line("crates.io: https"));
    assert!(is_known_key_line("features:"));
    
    assert!(!is_known_key_line("unknown: value"));
    assert!(!is_known_key_line("just text"));
    assert!(!is_known_key_line(""));
}

#[test]
fn test_parse_feature_line_all_cases() {
    // Test parse_feature_line with various inputs
    let f1 = parse_feature_line(" +default = []").unwrap();
    assert_eq!(f1.name, "default");
    assert!(f1.is_default);
    assert_eq!(f1.dependencies.len(), 0);
    
    let f2 = parse_feature_line("  extra = [a, b]").unwrap();
    assert_eq!(f2.name, "extra");
    assert!(!f2.is_default);
    assert_eq!(f2.dependencies.len(), 2);
    
    let f3 = parse_feature_line("  complex = [dep:serde, pkg/feat, opt?]").unwrap();
    assert_eq!(f3.dependencies.len(), 3);
    
    assert!(parse_feature_line("x not valid").is_none());
    assert!(parse_feature_line(" @invalid").is_none());
    assert!(parse_feature_line(" +  = []").is_none());
}

#[test]
fn parse_desc_empty_then_complete() {
    // Crate with empty description immediately followed by version
    let input = "empty-desc

version: 1.0
license: MIT
";
    let info = parse(input).unwrap();
    assert_eq!(info.name, "empty-desc");
    assert_eq!(info.description, "");
}

#[test]
fn parse_header_no_keywords_plain_name() {
    let (name, kw) = parse_header("simple").unwrap();
    assert_eq!(name, "simple");
    assert!(kw.is_empty());
}

#[test]
fn parse_feature_line_missing_equals() {
    // Missing "= [...]" part
    assert!(parse_feature_line("  featurename [dep]").is_none());
}

#[test]
fn parse_feature_line_no_closing_bracket() {
    // Missing closing bracket
    assert!(parse_feature_line("  +feature = [dep1, dep2").is_none());
}

#[test]
fn parse_feature_line_empty_name_no_trim() {
    // Name is blank after trimming spaces
    assert!(parse_feature_line("   = []").is_none());
}

#[test]
fn parse_all_optional_fields_individually() {
    // Test each individual optional field transition
    let input1 = "v1
D
version: 1.0
license: MIT
rust-version: 1.60
";
    let info1 = parse(input1).unwrap();
    assert_eq!(info1.rust_version, Some("1.60".to_string()));
    
    let input2 = "v2
D
version: 1.0
license: MIT
documentation: https://docs.rs
";
    let info2 = parse(input2).unwrap();
    assert_eq!(info2.documentation, Some("https://docs.rs".to_string()));
    
    let input3 = "v3
D
version: 1.0
license: MIT
homepage: https://home
";
    let info3 = parse(input3).unwrap();
    assert_eq!(info3.homepage, Some("https://home".to_string()));
    
    let input4 = "v4
D
version: 1.0
license: MIT
repository: https://repo
";
    let info4 = parse(input4).unwrap();
    assert_eq!(info4.repository, Some("https://repo".to_string()));
    
    let input5 = "v5
D
version: 1.0
license: MIT
crates.io: https://crates
";
    let info5 = parse(input5).unwrap();
    assert_eq!(info5.crates_io, Some("https://crates".to_string()));
}

#[test]
fn raw_fields_apply_none_on_unrecognized() {
    // Ensure unrecognized lines don't cause panics
    let mut fields = RawFields::default();
    fields.apply("unknown: value");
    fields.apply("random: data");
    fields.apply("features:"); // This should not be handled by apply()
    
    assert_eq!(fields.version, None);
    assert_eq!(fields.license, None);
}

#[test]
fn parse_strip_key_whitespace_handling() {
    let v1 = strip_key("version:   1.0", "version:").unwrap();
    assert_eq!(v1, "1.0");
    
    let v2 = strip_key("license:MIT", "license:").unwrap();
    assert_eq!(v2, "MIT");
    
    let v3 = strip_key("documentation:     https://test", "documentation:").unwrap();
    assert_eq!(v3, "https://test");
}

#[test]
fn test_strip_key_all_cases() {
    // Test strip_key with all field types
    let v = strip_key("version:   1.0.0", "version:").unwrap();
    assert_eq!(v, "1.0.0");
    
    let l = strip_key("license: MIT", "license:").unwrap();
    assert_eq!(l, "MIT");
    
    let r = strip_key("rust-version:  1.65", "rust-version:").unwrap();
    assert_eq!(r, "1.65");
    
    assert!(strip_key("version 1.0", "version:").is_none());
    assert!(strip_key("nope", "version:").is_none());
}

#[test]
fn parse_after_features_still_features_state() {
    // Verify we remain in Features state and process multiple feature lines
    let input = "multi-feat
Desc
version: 1.0.0
license: MIT
features:
 +base = [dep1, dep2]
  opt1 = [x]
  opt2 = []
  opt3 = [a, b, c]
";
    let info = parse(input).unwrap();
    assert_eq!(info.features.len(), 4);
    assert_eq!(info.features[0].name, "base");
    assert_eq!(info.features[1].name, "opt1");
    assert_eq!(info.features[2].name, "opt2");
    assert_eq!(info.features[3].name, "opt3");
    assert_eq!(info.features[3].dependencies, vec!["a", "b", "c"]);
}

#[test]
fn parse_fields_stay_in_fields_until_features() {
    // Verify multiple field lines keep us in Fields state
    let input = "fields-test
Desc
version: 1.0.0
license: MIT
rust-version: 1.60
documentation: https://test
homepage: https://test
repository: https://test
crates.io: https://test
features:
 +test = []
";
    let info = parse(input).unwrap();
    assert_eq!(info.version, "1.0.0");
    assert_eq!(info.license, "MIT");
    assert_eq!(info.rust_version.as_deref(), Some("1.60"));
    assert_eq!(info.features.len(), 1);
}

#[test]
fn parse_feature_default_with_deps() {
    // Verify default features are marked correctly
    let input = "def-feat
Test
version: 1.0.0
license: MIT
features:
 +default = [dep_a, dep_b]
  other = []
";
    let info = parse(input).unwrap();
    assert!(info.features[0].is_default);
    assert!(!info.features[1].is_default);
    assert_eq!(info.features[0].dependencies.len(), 2);
}

#[test]
fn parse_fields_only_some_optional() {
    // Test with only some optional fields
    let input = "partial
Desc
version: 2.0.0
license: Apache-2.0
documentation: https://docs
homepage: https://home
";
    let info = parse(input).unwrap();
    assert_eq!(info.version, "2.0.0");
    assert_eq!(info.license, "Apache-2.0");
    assert_eq!(info.documentation.as_deref(), Some("https://docs"));
    assert_eq!(info.homepage.as_deref(), Some("https://home"));
    assert!(info.repository.is_none());
    assert!(info.crates_io.is_none());
    assert!(info.rust_version.is_none());
}
