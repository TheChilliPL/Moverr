use embed_manifest::manifest::ExecutionLevel;
use embed_manifest::manifest::MaxVersionTested::Windows11Version22H2;
use embed_manifest::manifest::SupportedOS::{Windows10, Windows8};
use embed_manifest::{embed_manifest, new_manifest};
use std::env::{var, var_os};

fn is_windows() -> bool {
    var_os("CARGO_CFG_WINDOWS").is_some()
}

fn is_release_build() -> bool {
    var("PROFILE").unwrap() == "release"
}

fn main() {
    if is_windows() {
        let mut manifest = new_manifest(&var("CARGO_PKG_NAME").unwrap())
            .supported_os(Windows8..=Windows10)
            .max_version_tested(Windows11Version22H2);
        if is_release_build() {
            manifest = manifest.requested_execution_level(ExecutionLevel::RequireAdministrator);
        }

        embed_manifest(manifest).expect("Failed to embed manifest!");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
