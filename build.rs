fn main() {
    let cargo_toml = std::fs::read_to_string("Cargo.toml").expect("reading Cargo.toml");
    let parsed: toml::Value = cargo_toml.parse().expect("parsing Cargo.toml");
    let wt = &parsed["dependencies"]["wasmtime"];
    let version = wt
        .as_str()
        .or_else(|| wt.get("version").and_then(|v| v.as_str()))
        .expect("wasmtime dependency should have a version (string or table with `version` key)");
    println!("cargo:rustc-env=WASMTIME_DEP_VERSION={version}");
    println!("cargo:rerun-if-changed=Cargo.toml");
}
