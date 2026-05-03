fn main() {
    let cargo_toml = std::fs::read_to_string("Cargo.toml").expect("reading Cargo.toml");
    let parsed: toml::Value = cargo_toml.parse().expect("parsing Cargo.toml");
    let version = parsed["dependencies"]["wasmtime"]
        .as_str()
        .expect("wasmtime dependency should be a plain version string");
    println!("cargo:rustc-env=WASMTIME_DEP_VERSION={version}");
    println!("cargo:rerun-if-changed=Cargo.toml");
}
