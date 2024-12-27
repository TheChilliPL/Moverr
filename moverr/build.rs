use embed_manifest::manifest::ExecutionLevel;
use embed_manifest::manifest::MaxVersionTested::Windows11Version22H2;
use embed_manifest::manifest::SupportedOS::{Windows10, Windows8};
use embed_manifest::{embed_manifest, new_manifest};
use std::env::{var, var_os};

fn main() {
    let is_windows = var_os("CARGO_CFG_WINDOWS").is_some();

    if is_windows {
        let manifest = new_manifest(&var("CARGO_PKG_NAME").unwrap())
            .supported_os(Windows8..=Windows10)
            .max_version_tested(Windows11Version22H2)
            .requested_execution_level(ExecutionLevel::RequireAdministrator);

        embed_manifest(manifest).expect("Failed to embed manifest!");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
