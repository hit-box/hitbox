//! Golden-bytes checks for stored layouts.
//!
//! Each check compares bytes with `tests/golden/v{WIRE_VERSION}/<name>`. Files in `v1/` may still
//! change until 0.3.0 ships; after that a changed layout needs a new `WIRE_VERSION` and directory.

use std::{env, fs, path::PathBuf};

use crate::WIRE_VERSION;

/// Set to rewrite the golden files from the current encoders.
const BLESS: &str = "HITBOX_BLESS_GOLDEN";

/// Asserts that `bytes` equal the golden file `name`, or writes it when [`BLESS`] is set.
pub(crate) fn assert_golden(name: &str, bytes: &[u8]) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/golden/v{WIRE_VERSION}"))
        .join(name);
    if env::var_os(BLESS).is_some() {
        fs::create_dir_all(path.parent().expect("golden files have a directory"))
            .expect("create the golden directory");
        fs::write(&path, bytes).expect("write the golden file");
        return;
    }
    let expected = fs::read(&path).unwrap_or_else(|err| {
        panic!(
            "{}: {err}; run the test with {BLESS}=1 to create it",
            path.display()
        )
    });
    assert!(
        expected == bytes,
        "the stored layout `{name}` changed: {} bytes now, {} in {}. If intended, rewrite it with \
         {BLESS}=1 before 0.3.0 ships; after that, raise WIRE_VERSION and add tests/golden/v{}/",
        bytes.len(),
        expected.len(),
        path.display(),
        WIRE_VERSION + 1,
    );
}
