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

// ── Typestate Pattern: Parser States ──────────────────────────────────────────

/// Marker type for parser in Description state.
///
/// Valid transitions: Description → Fields or Description → Features
struct ParsingDescription;

/// Marker type for parser in Fields state (key-value pairs).
///
/// Valid transitions: Fields → Features or stay in Fields
struct ParsingFields;

/// Marker type for parser in Features state.
///
/// Valid transitions: Features → Complete (terminal state)
struct ParsingFeatures;

/// Marker type for completed parser (no more transitions).
struct ParsingComplete;

/// A parser in a specific state, parameterized by a marker type.
///
/// This type encodes the current parsing state at the type level, ensuring
/// that only valid state transitions are possible. The `parse()` function
/// gradually constructs a `Parser<ParsingComplete>` by consuming and
/// transitioning through intermediate states.
struct Parser<S> {
    state_marker: std::marker::PhantomData<S>,
    name: String,
    keywords: Vec<String>,
    description_parts: Vec<String>,
    fields: RawFields,
    features: Vec<Feature>,
}

impl Parser<ParsingDescription> {
    /// Creates a new parser in the Description state.
    fn new(name: String, keywords: Vec<String>) -> Self {
        Parser {
            state_marker: std::marker::PhantomData,
            name,
            keywords,
            description_parts: Vec::new(),
            fields: RawFields::default(),
            features: Vec::new(),
        }
    }

    /// Process one line in the Description state.
    ///
    /// Returns either:
    /// - `ParserOrFields::Description` if we remain in Description state
    /// - `ParserOrFields::Fields` if we transition to Fields state
    ///
    /// Note: Direct transition to Features state is impossible since version/license
    /// (required fields) must be encountered first, triggering a transition to Fields.
    fn process_line(mut self, line: &str) -> ParserOrFields {
        if is_known_key_line(line) {
            self.fields.apply(line);
            ParserOrFields::Fields(self.transition_to_fields())
        } else {
            self.description_parts.push(line.to_string());
            ParserOrFields::Description(self)
        }
    }

    /// Transition from Description state to Fields state.
    fn transition_to_fields(self) -> Parser<ParsingFields> {
        Parser {
            state_marker: std::marker::PhantomData,
            name: self.name,
            keywords: self.keywords,
            description_parts: self.description_parts,
            fields: self.fields,
            features: self.features,
        }
    }
}

impl Parser<ParsingFields> {
    /// Process one line in the Fields state.
    ///
    /// Returns either:
    /// - `ParserOrFeatures::Fields` if we remain in Fields state
    /// - `ParserOrFeatures::Features` if we transition to Features state
    fn process_line(mut self, line: &str) -> ParserOrFeatures {
        if line.starts_with("features:") {
            ParserOrFeatures::Features(self.transition_to_features())
        } else {
            self.fields.apply(line);
            ParserOrFeatures::Fields(self)
        }
    }

    /// Transition from Fields state to Features state.
    fn transition_to_features(self) -> Parser<ParsingFeatures> {
        Parser {
            state_marker: std::marker::PhantomData,
            name: self.name,
            keywords: self.keywords,
            description_parts: self.description_parts,
            fields: self.fields,
            features: self.features,
        }
    }
}

impl Parser<ParsingFeatures> {
    /// Process one line in the Features state.
    ///
    /// Always remains in Features state (terminal state before completion).
    fn process_line(mut self, line: &str) -> Self {
        if let Some(feature) = parse_feature_line(line) {
            self.features.push(feature);
        }
        // Non-matching lines inside the features block are silently ignored
        self
    }

    /// Transition from Features state to Complete state.
    fn complete(self) -> Result<Parser<ParsingComplete>, ParseError> {
        Ok(Parser {
            state_marker: std::marker::PhantomData,
            name: self.name,
            keywords: self.keywords,
            description_parts: self.description_parts,
            fields: self.fields,
            features: self.features,
        })
    }
}

impl Parser<ParsingComplete> {
    /// Extract the final `CrateInfo` from a completed parser.
    fn into_crate_info(self) -> Result<CrateInfo, ParseError> {
        let description = self.description_parts.join("\n");
        let version = self.fields.version.ok_or(ParseError::MissingField("version"))?;
        let license = self.fields.license.ok_or(ParseError::MissingField("license"))?;

        Ok(CrateInfo {
            name: self.name,
            keywords: self.keywords,
            description,
            version,
            license,
            rust_version: self.fields.rust_version,
            documentation: self.fields.documentation,
            homepage: self.fields.homepage,
            repository: self.fields.repository,
            crates_io: self.fields.crates_io,
            features: self.features,
        })
    }
}

/// Result type for transitions from Description state (only to Description or Fields).
enum ParserOrFields {
    Description(Parser<ParsingDescription>),
    Fields(Parser<ParsingFields>),
}

/// Result type for transitions that could go to either Fields or Features.
enum ParserOrFeatures {
    Fields(Parser<ParsingFields>),
    Features(Parser<ParsingFeatures>),
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

    // ── Phase 1: Parse header ─────────────────────────────────────────────────
    let header_line = lines.next().ok_or(ParseError::Empty)?;
    let (name, keywords) = parse_header(header_line)?;

    // ── Phase 2-4: State machine processing ───────────────────────────────────
    // Create the initial parser in Description state
    let parser = Parser::<ParsingDescription>::new(name, keywords);

    // Feed remaining lines through the state machine
    let completed = lines.fold(
        ParserState::Description(parser),
        |parser_state, line| {
            match parser_state {
                ParserState::Description(p) => match p.process_line(line) {
                    ParserOrFields::Description(p) => ParserState::Description(p),
                    ParserOrFields::Fields(p) => ParserState::Fields(p),
                },
                ParserState::Fields(p) => match p.process_line(line) {
                    ParserOrFeatures::Fields(p) => ParserState::Fields(p),
                    ParserOrFeatures::Features(p) => ParserState::Features(p),
                },
                ParserState::Features(p) => ParserState::Features(p.process_line(line)),
            }
        },
    );

    // Complete the parser and extract the final result
    match completed {
        ParserState::Description(p) => p.transition_to_fields().transition_to_features().complete()?.into_crate_info(),
        ParserState::Fields(p) => p.transition_to_features().complete()?.into_crate_info(),
        ParserState::Features(p) => p.complete()?.into_crate_info(),
    }
}

/// Represents a parser in any of its valid states.
///
/// This enum allows us to track which state the parser is currently in during
/// the fold operation. Only valid transitions are possible because transforming
/// from one ParserState variant to another is type-safe.
enum ParserState {
    Description(Parser<ParsingDescription>),
    Fields(Parser<ParsingFields>),
    Features(Parser<ParsingFeatures>),
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Parses the first line of `cargo info` output into `(name, keywords)`.
///
/// Input format: `<name>[ #<kw1> #<kw2> ...]`
pub fn parse_header(line: &str) -> Result<(String, Vec<String>), ParseError> {
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
pub fn is_known_key_line(line: &str) -> bool {
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
///
/// This structure holds all the optional metadata fields that may appear in
/// `cargo info` output. Fields that do not appear in the output will be `None`.
#[derive(Default)]
pub struct RawFields {
    /// The published version (e.g., `"1.0.0"` or `"2.0.117"`).
    pub version: Option<String>,

    /// The SPDX license expression (e.g., `"MIT OR Apache-2.0"`).
    pub license: Option<String>,

    /// The minimum supported Rust version (e.g., `"1.71"`).
    pub rust_version: Option<String>,

    /// The URL to the crate's API documentation.
    pub documentation: Option<String>,

    /// The URL to the crate's project homepage.
    pub homepage: Option<String>,

    /// The URL to the crate's source code repository.
    pub repository: Option<String>,

    /// The URL to the crate's page on crates.io.
    pub crates_io: Option<String>,
}

impl RawFields {
    /// Recognizes and stores a single `key: value` line.
    pub fn apply(&mut self, line: &str) {
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
pub fn strip_key(line: &str, prefix: &str) -> Option<String> {
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
pub fn parse_feature_line(line: &str) -> Option<Feature> {
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
