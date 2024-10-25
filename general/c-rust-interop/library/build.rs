// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Build script for the `sample-wdm-driver` crate.
//!
//! Based on the [`wdk_build::Config`] parsed from the build tree, this build
//! script will provide `Cargo` with the necessary information to build the
//! driver binary (ex. linker flags)

use std::env;
extern crate cbindgen;

fn main() -> anyhow::Result<()> {
    wdk_build::configure_wdk_library_build()?;

    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let builder = cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(cbindgen::Language::C)
        .with_no_includes()
        .with_header(
            r#"// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0"#,
        )
        .generate()
        .expect("Unable to generate bindings");

    builder.write_to_file("bindings.h");

    Ok(())
}
