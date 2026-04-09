# cargo_info_types

Parse the output of [`cargo info`](https://doc.rust-lang.org/cargo/commands/cargo-info.html) into well-typed Rust structures.

[![Crates.io](https://img.shields.io/crates/v/cargo_info_types.svg)](https://crates.io/crates/cargo_info_types)
[![Docs.rs](https://docs.rs/cargo_info_types/badge.svg)](https://docs.rs/cargo_info_types)
[![License](https://img.shields.io/crates/l/cargo_info_types.svg)](https://github.com/dtolnay/cargo_info_types#license)

## Overview

`cargo_info_types` provides a lightweight library to parse the human-readable output of the `cargo info` command into structured Rust types.

### What it does

- **Parses `cargo info` output** into strongly-typed [`CrateInfo`] structures
- **Extracts metadata** like version, license, dependencies, MSRV, documentation URLs
- **Parses feature flags** with their dependencies in a structured format
- **Strips ANSI escape sequences** for cases where `--color never` cannot be used
- **Optional command execution** with the `execute` feature (runs `cargo info` directly)

## Quick Start

### Parse existing output

```rust
use cargo_info_types::parse;

let output = "syn #macros #syn
Parser for Rust source code
version: 2.0.117
license: MIT OR Apache-2.0
rust-version: 1.71
documentation: https://docs.rs/syn
repository: https://github.com/dtolnay/syn
crates.io: https://crates.io/crates/syn/2.0.117
features:
 +default      = [derive, parsing]
  derive       = []
  parsing      = []
";

let info = parse(output).unwrap();
assert_eq!(info.name, "syn");
assert_eq!(info.version, "2.0.117");
assert_eq!(info.keywords, vec!["macros", "syn"]);
assert!(info.features[0].is_default);
```

### Run `cargo info` directly (requires `execute` feature)

```toml
[dependencies]
cargo_info_types = { version = "0.1", features = ["execute"] }
```

```rust
use cargo_info_types::execute;

let info = execute("syn").unwrap();
println!("Version: {}", info.version);
println!("MSRV: {:?}", info.rust_version);
```

### Handle ANSI color codes

```rust
use cargo_info_types::{strip_ansi_escapes, parse};

// If you can't use --color never
let colored_output = "\x1b[1;32msyn\x1b[0m #macros\n...";
let clean = strip_ansi_escapes(colored_output);
let info = parse(&clean).unwrap();
```

## API Overview

### Main Types

- **[`CrateInfo`]** — Top-level structure containing all information about a crate
  - `name: String` — The crate name
  - `version: String` — Published version (required)
  - `license: String` — SPDX license expression (required)
  - `keywords: Vec<String>` — Associated keywords
  - `description: String` — Short description from crates.io
  - `rust_version: Option<String>` — Minimum supported Rust version
  - `documentation: Option<String>` — URL to docs
  - `homepage: Option<String>` — Project homepage URL
  - `repository: Option<String>` — Source repository URL
  - `crates_io: Option<String>` — Crates.io URL
  - `features: Vec<Feature>` — Available feature flags

- **[`Feature`]** — A single Cargo feature flag
  - `is_default: bool` — Whether this is a default feature
  - `name: String` — Feature name
  - `dependencies: Vec<String>` — Features/dependencies it enables

### Main Functions

- **[`parse(input: &str)`]** — Parse `cargo info` output into a [`CrateInfo`]
- **[`execute(crate_name: &str)`]** — Run `cargo info` and parse the result *(feature: `execute`)*
- **[`strip_ansi_escapes(input: &str)`]** — Remove ANSI/VT100 color sequences
- **[`parse_header(line: &str)`]** — Parse the first line (name + keywords)
- **[`parse_feature_line(line: &str)`]** — Parse a single feature line
- **[`is_known_key_line(line: &str)`]** — Check if a line is a recognized field

### Error Types

- **[`ParseError`]** — Errors from parsing
  - `Empty` — Input was empty
  - `InvalidHeader` — First line could not be parsed
  - `MissingField` — Required field (version/license) absent

- **[`ExecuteError`]** — Errors from command execution *(feature: `execute`)*
  - `Io` — Failed to run `cargo info`
  - `CargoError` — `cargo info` exited with error
  - `Parse` — Output parsing failed

## Feature Flags

| Feature | Description |
|---------|-------------|
| `execute` | Enables the [`execute`] function, which spawns `cargo info` as a subprocess. Zero additional dependencies. Default: disabled |

## Architecture

The parser uses a **typestate pattern** to ensure state transitions are correct at compile time:

```
Input
  ↓
Header (parse name + keywords)
  ↓
Description state (accumulate description lines)
  ↓
Fields state (parse version, license, metadata)
  ↓
Features state (parse feature flags)
  ↓
Complete (extract final CrateInfo)
```

Each state transition is type-safe, preventing invalid states from being represented in the type system.

## Examples

### Extract all optional metadata

```rust
use cargo_info_types::parse;

let info = parse(/* ... */).unwrap();

if let Some(msrv) = info.rust_version {
    println!("MSRV: {}", msrv);
}

if let Some(repo) = info.repository {
    println!("Repository: {}", repo);
}

for feature in &info.features {
    if feature.is_default {
        println!("Default feature: {}", feature.name);
    }
}
```

### Get dependency information from a feature

```rust
use cargo_info_types::parse;

let info = parse(/* ... */).unwrap();

for feature in &info.features {
    if feature.name == "advanced" {
        println!("'advanced' enables: {:?}", feature.dependencies);
        // Example dependencies:
        // - "expensive-dep" (optional dependency)
        // - "dep:futures" (enables futures dependency)
        // - "tokio/macros" (enables feature in dependency)
        // - "tokio?/macros" (optional dependency feature)
    }
}
```

## Format Specification

The expected format of `cargo info` output is:

```text
<name> [#<keyword> ...]
<description lines...>
version: <version>
license: <license>
[rust-version: <msrv>]
[documentation: <url>]
[homepage: <url>]
[repository: <url>]
[crates.io: <url>]
[features:
 +<name> = [<dep>, ...]
  <name> = []
  ...]
```

**Required fields:** `version`, `license`

**Optional fields:** Description, rust-version, documentation, homepage, repository, crates.io, features
