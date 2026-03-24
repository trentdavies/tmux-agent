#[path = "src/versioning.rs"]
mod versioning;

use std::env;
use std::path::Path;
use std::process::Command;

use versioning::{format_long_version, format_version, parse_git_describe, GitVersion};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/versioning.rs");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("missing CARGO_MANIFEST_DIR");
    let manifest_dir = Path::new(&manifest_dir);
    let package_version = env::var("CARGO_PKG_VERSION").expect("missing CARGO_PKG_VERSION");

    emit_git_rerun_hints(manifest_dir);

    let version = compute_version(manifest_dir, &package_version);

    println!("cargo:rustc-env=TA_BUILD_VERSION={}", version.version);
    println!(
        "cargo:rustc-env=TA_BUILD_LONG_VERSION={}",
        version.long_version
    );
}

struct BuildVersion {
    version: String,
    long_version: String,
}

fn compute_version(manifest_dir: &Path, package_version: &str) -> BuildVersion {
    let describe = run_git(
        manifest_dir,
        &[
            "describe",
            "--tags",
            "--match",
            "v[0-9]*",
            "--long",
            "--dirty",
            "--always",
            "--abbrev=7",
        ],
    );

    let commit_count = run_git(manifest_dir, &["rev-list", "--count", "HEAD"])
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);

    let hash = run_git(manifest_dir, &["rev-parse", "--short=7", "HEAD"])
        .unwrap_or_else(|| "unknown".to_string());

    let git_version = describe
        .as_deref()
        .and_then(|value| parse_git_describe(value, package_version, commit_count))
        .unwrap_or_else(|| GitVersion::fallback(package_version, commit_count, hash));

    let version = format_version(&git_version);
    let long_version = format_long_version(&git_version, &version);

    BuildVersion {
        version,
        long_version,
    }
}

fn emit_git_rerun_hints(manifest_dir: &Path) {
    for path in ["HEAD", "index", "packed-refs"] {
        if let Some(git_path) = run_git(manifest_dir, &["rev-parse", "--git-path", path]) {
            println!("cargo:rerun-if-changed={}", git_path);
        }
    }

    if let Some(git_dir) = run_git(manifest_dir, &["rev-parse", "--git-dir"]) {
        println!("cargo:rerun-if-changed={git_dir}/refs");
    }
}

fn run_git(manifest_dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(manifest_dir)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let trimmed = stdout.trim();

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
