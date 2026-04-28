fn main() {
    // Generate C header when ffi-c feature is enabled
    #[cfg(feature = "ffi-c")]
    {
        let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        cbindgen::Builder::new()
            .with_crate(crate_dir)
            .with_language(cbindgen::Language::C)
            .with_include_guard("FARISLAND_METROLOGY_H")
            .with_documentation(true)
            .with_style(cbindgen::Style::Both)
            .generate()
            .expect("Unable to generate C bindings")
            .write_to_file("farisland_metrology.h");
    }

    // Generate uniffi scaffolding when ffi-uniffi feature is enabled
    #[cfg(feature = "ffi-uniffi")]
    {
        uniffi::generate_scaffolding("src/farisland_metrology.udl")
            .expect("Unable to generate uniffi scaffolding");
    }
}
