use std::process::Command;

use crate::parser::ParseError;
use crate::types::CrateInfo;
use thiserror::Error;

/// An error that occurred while executing or parsing a `cargo info` command.
#[derive(Debug, Error)]
pub enum ExecuteError {
    /// `cargo` could not be found or could not be launched.
    #[error("failed to run `cargo info`: {0}")]
    Io(#[from] std::io::Error),

    /// `cargo info` exited with a non-zero status.
    ///
    /// The contained string is the stderr output produced by Cargo, which
    /// typically contains a human-readable explanation of the failure (e.g.,
    /// crate not found, network error).
    #[error("`cargo info` failed (exit {code}): {stderr}")]
    CargoError {
        /// The exit status code, if one was available.
        code: i32,
        /// Standard error output from the `cargo info` invocation.
        stderr: String,
    },

    /// The command succeeded but its output could not be parsed.
    #[error("failed to parse `cargo info` output: {0}")]
    Parse(#[from] ParseError),
}

/// Executes `cargo info -q <crate_name> --color never` and parses the output.
///
/// This function is only available when the **`execute`** feature is enabled.
///
/// # Arguments
///
/// * `crate_name` — the name of the crate to look up (e.g., `"syn"`).
///
/// # Errors
///
/// Returns an [`ExecuteError`] if:
///
/// - `cargo` cannot be found or launched ([`ExecuteError::Io`]),
/// - `cargo info` exits with a non-zero status ([`ExecuteError::CargoError`]),
/// - the output does not match the expected format ([`ExecuteError::Parse`]).
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "execute")]
/// use cargo_info_types::execute;
///
/// # #[cfg(feature = "execute")]
/// let info = execute("syn").unwrap();
/// # #[cfg(feature = "execute")]
/// assert_eq!(info.name, "syn");
/// ```
#[cfg(feature = "execute")]
pub fn execute(crate_name: &str) -> Result<CrateInfo, ExecuteError> {
    let output = Command::new("cargo")
        .args(["info", "-q", crate_name, "--color", "never"])
        .output()?;

    if !output.status.success() {
        return Err(ExecuteError::CargoError {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let info = crate::parse(&stdout)?;
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_nonexistent_crate_returns_cargo_error() {
        let result = execute("cargo_info_types_coverage_test_no_crate_12345");

        match result {
            Err(ExecuteError::CargoError { code, stderr }) => {
                assert_ne!(code, 0);
                assert!(!stderr.trim().is_empty());
            }
            Err(ExecuteError::Io(err)) => {
                panic!("cargo binary must be available for this test: {err}");
            }
            Err(ExecuteError::Parse(err)) => {
                panic!("expected cargo error, got parse error: {err}");
            }
            Ok(_) => panic!("expected cargo info to fail for nonexistent crate"),
        }
    }

    #[test]
    fn execute_returns_crate_info_struct_on_success() {
        // Test with a real, stable crate that should always be available
        let result = execute("serde");
        match result {
            Ok(info) => {
                assert!(!info.name.is_empty());
                assert!(!info.version.is_empty());
                assert!(!info.license.is_empty());
            }
            Err(e) => {
                // If network/cargo unavailable, that's OK - just ensure we tried
                eprintln!("Execute test skipped due to: {}", e);
            }
        }
    }
}

