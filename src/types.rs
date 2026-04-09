/// Information about a crate as reported by `cargo info`.
///
/// This is the top-level type returned by [`crate::parse`]. It corresponds to
/// one invocation of `cargo info -q <crate-name> --color never`.
///
/// # Example output
///
/// ```text
/// syn #macros #syn
/// Parser for Rust source code
/// version: 2.0.117
/// license: MIT OR Apache-2.0
/// rust-version: 1.71
/// documentation: https://docs.rs/syn
/// repository: https://github.com/dtolnay/syn
/// crates.io: https://crates.io/crates/syn/2.0.117
/// features:
///  +default      = [derive, parsing, printing, clone-impls, proc-macro]
///   clone-impls  = []
///   ...
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateInfo {
    /// The name of the crate.
    pub name: String,

    /// Keywords associated with the crate.
    ///
    /// These are the `#keyword` tokens that appear on the first line of `cargo
    /// info` output, directly after the crate name. For example, the output
    /// `syn #macros #syn` yields `["macros", "syn"]`.
    pub keywords: Vec<String>,

    /// A short description of the crate, as listed on crates.io.
    ///
    /// May span multiple lines if the description is long enough to be wrapped
    /// by `cargo info`.
    pub description: String,

    /// The published version of the crate (e.g., `"2.0.117"` or `"1.0.0-alpha.3"`).
    pub version: String,

    /// The SPDX license expression under which the crate is published
    /// (e.g., `"MIT OR Apache-2.0"`).
    pub license: String,

    /// The minimum supported Rust version (MSRV), if specified by the crate.
    ///
    /// Stored as a plain string (e.g., `"1.71"`).
    pub rust_version: Option<String>,

    /// The URL to the crate's API documentation, if specified.
    pub documentation: Option<String>,

    /// The URL to the crate's project homepage, if specified.
    pub homepage: Option<String>,

    /// The URL to the crate's source repository, if specified.
    pub repository: Option<String>,

    /// The URL to the crate's page on crates.io, if available.
    pub crates_io: Option<String>,

    /// The Cargo feature flags declared by the crate.
    ///
    /// Features prefixed with `+` in `cargo info` output (i.e., the default
    /// feature set) have [`Feature::is_default`] set to `true`.
    pub features: Vec<Feature>,
}

/// A Cargo feature flag declared by a crate.
///
/// Each feature may depend on other features from the same crate or from
/// dependency crates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feature {
    /// Whether this feature is part of the crate's default feature set.
    ///
    /// `cargo info` marks default features with a `+` prefix; this field
    /// reflects that annotation.
    pub is_default: bool,

    /// The name of the feature.
    pub name: String,

    /// The list of features or dependencies that this feature enables.
    ///
    /// Entries may take the following forms:
    ///
    /// | Form | Meaning |
    /// |------|---------|
    /// | `"some_feature"` | Another feature of the same crate |
    /// | `"dep:some_crate"` | Enables an optional dependency |
    /// | `"crate_name/feature_name"` | Enables a feature of a dependency |
    /// | `"crate_name?/feature_name"` | Enables a feature of an *optional* dependency |
    pub dependencies: Vec<String>,
}
