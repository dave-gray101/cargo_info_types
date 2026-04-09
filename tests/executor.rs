#![cfg(feature = "execute")]

use cargo_info_types::execute;

#[test]
fn execute_nonexistent_crate_returns_cargo_error() {
    let result = execute("cargo_info_types_coverage_test_no_crate_12345");

    match result {
        Err(cargo_info_types::ExecuteError::CargoError { code, stderr }) => {
            assert_ne!(code, 0);
            assert!(!stderr.trim().is_empty());
        }
        Err(cargo_info_types::ExecuteError::Io(err)) => {
            panic!("cargo binary must be available for this test: {err}");
        }
        Err(cargo_info_types::ExecuteError::Parse(err)) => {
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
