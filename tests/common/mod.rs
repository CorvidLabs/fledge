//! Shared helpers for integration tests. Per-module test files at
//! `tests/<module>.rs` pull these in via `mod common; use common::*;`.

#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

pub fn cargo_bin() -> String {
    env!("CARGO_BIN_EXE_fledge").to_string()
}

pub fn run_fledge(args: &[&str]) -> std::process::Output {
    let bin = cargo_bin();
    Command::new(&bin).args(args).output().unwrap()
}

pub fn run_fledge_in(dir: &Path, args: &[&str]) -> std::process::Output {
    let bin = cargo_bin();
    Command::new(&bin)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

/// Run fledge with HOME pointed at a fresh tempdir so the invocation sees an
/// empty plugin registry.  The caller owns the `TempDir` — keep it in scope
/// for the duration of the test so the directory is not removed early.
pub fn run_fledge_isolated(args: &[&str], home: &tempfile::TempDir) -> std::process::Output {
    let bin = cargo_bin();
    Command::new(&bin)
        .args(args)
        .env("HOME", home.path())
        .output()
        .unwrap()
}
