// Diagnostics change between compiler releases, so the `.stderr` snapshots
// are recorded with the MSRV: `cargo +1.91.1 test -p hitbox-fn --test compile_fail`.
#[rustversion::attr(not(stable(1.91)), ignore = "snapshots are recorded with the MSRV")]
#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();

    // Feature-independent tests
    t.compile_fail("tests/compile_fail/sync_fn.rs");
    t.compile_fail("tests/compile_fail/self_method.rs");
    t.compile_fail("tests/compile_fail/skip_unknown_param.rs");

    // CacheableResponse tests produce different errors with rkyv_format
    #[cfg(not(feature = "rkyv_format"))]
    {
        t.compile_fail("tests/compile_fail/cached_field_no_clone.rs");
        t.compile_fail("tests/compile_fail/skip_field_no_default.rs");
    }
    #[cfg(feature = "rkyv_format")]
    {
        t.compile_fail("tests/compile_fail/cached_field_no_clone_rkyv.rs");
        t.compile_fail("tests/compile_fail/skip_field_no_default_rkyv.rs");
    }
}
