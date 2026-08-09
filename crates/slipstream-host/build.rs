//! Build script: stamps the build version.
fn main() {
    // Build provenance: stamp the exact package/build version into the binary so a running host
    // can report what it is (mgmt /health, the startup log, `--version`) and a stale/shadowed
    // install is detectable. CI (deb.yml / rpm.yml / the RPM spec / the PKGBUILD) sets
    // SLIPSTREAM_BUILD_VERSION to the full package version (e.g. `0.2.0~ci120.g802e98d`); a plain
    // `cargo build` falls back to the crate version. Deliberately NOT git-derived — the RPM builds
    // from a `git archive` tarball with no .git, and a hard git dependency would break it.
    let version = std::env::var("SLIPSTREAM_BUILD_VERSION")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".into()));
    println!("cargo:rustc-env=SLIPSTREAM_VERSION={version}");
    println!("cargo:rerun-if-env-changed=SLIPSTREAM_BUILD_VERSION");
}
