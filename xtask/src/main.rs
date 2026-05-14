//! `cargo xtask` — midstream task runner.
//!
//! Implements ADR-0037 as the canonical home for the chore commands
//! that used to live in root-level shell scripts (`publish_*.sh`,
//! `install.sh`, `setup.sh`). Adding a new subcommand here keeps it
//! cross-platform, testable, and discoverable via `cargo xtask
//! --help`.
//!
//! Invocation:
//!
//! ```bash
//! cargo xtask ci          # run the same gates CI runs
//! cargo xtask deny        # cargo audit + cargo deny check
//! cargo xtask publish-check  # cargo publish --dry-run for every crate
//! cargo xtask wasm        # wasm-pack build the wasm crates
//! ```
//!
//! `cargo xtask` is wired into `.cargo/config.toml` as an alias, so
//! contributors invoke it from anywhere in the workspace.

use anyhow::Result;
use clap::{Parser, Subcommand};
use xshell::{cmd, Shell};

#[derive(Parser)]
#[command(
    name = "xtask",
    bin_name = "cargo xtask",
    about = "midstream task runner (ADR-0037)",
    long_about = None,
    version
)]
struct Xtask {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the same gates CI runs locally (format, clippy, MSRV check, tests).
    ///
    /// Mirrors `.github/workflows/rust-ci.yml`. Use this before pushing
    /// to spot anything CI would catch.
    Ci,

    /// Run the supply-chain audit gates: `cargo audit` + `cargo deny check`.
    ///
    /// Mirrors `.github/workflows/audit.yml`. Requires `cargo-audit`
    /// and `cargo-deny` installed (see `cargo xtask install`).
    Deny,

    /// `cargo publish --dry-run` for every publishable workspace crate
    /// in dependency order. Catches version-spec / manifest issues
    /// before tagging a release.
    PublishCheck,

    /// `wasm-pack build` for the WASM crates (web + bundler + nodejs
    /// targets). Requires `wasm-pack` installed.
    Wasm,

    /// Install the dev-machine toolchain bits this repo needs:
    /// `cargo-audit`, `cargo-deny`, `cargo-edit`, `wasm-pack`. Idempotent.
    Install,
}

fn main() -> Result<()> {
    let args = Xtask::parse();
    let sh = Shell::new()?;
    // Run every subcommand from the workspace root so paths in
    // shell-outs are stable regardless of where the user invoked
    // `cargo xtask` from. CARGO_MANIFEST_DIR points to xtask/, so
    // step one above.
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    sh.change_dir(format!("{manifest_dir}/.."));

    match args.cmd {
        Cmd::Ci => run_ci(&sh),
        Cmd::Deny => run_deny(&sh),
        Cmd::PublishCheck => run_publish_check(&sh),
        Cmd::Wasm => run_wasm(&sh),
        Cmd::Install => run_install(&sh),
    }
}

/// Local-CI gate. Mirrors `.github/workflows/rust-ci.yml`.
///
/// `cargo check` and `cargo test` exclude the legacy `midstream` (the
/// example tests reference renamed types per ADR-0005 dedup) and the
/// long-retired `hyprstream` (per ADR-0002).
fn run_ci(sh: &Shell) -> Result<()> {
    println!("--> cargo fmt --all -- --check");
    cmd!(sh, "cargo fmt --all -- --check").run()?;

    println!("--> cargo clippy --workspace --all-targets -- -D warnings");
    cmd!(
        sh,
        "cargo clippy --workspace --exclude midstream --all-targets -- -D warnings"
    )
    .run()?;

    println!("--> cargo check --workspace --locked");
    cmd!(
        sh,
        "cargo check --workspace --exclude midstream --exclude hyprstream --locked"
    )
    .run()?;

    println!("--> cargo test --workspace --lib --locked");
    cmd!(
        sh,
        "cargo test --workspace --exclude midstream --exclude hyprstream --lib --locked"
    )
    .run()?;

    println!("\nci ok");
    Ok(())
}

/// Supply-chain audit. Mirrors `.github/workflows/audit.yml`.
fn run_deny(sh: &Shell) -> Result<()> {
    println!("--> cargo audit");
    cmd!(sh, "cargo audit").run()?;

    println!("--> cargo deny check");
    cmd!(sh, "cargo deny check").run()?;

    println!("\ndeny ok");
    Ok(())
}

/// Topological-order `cargo publish --dry-run` over the publishable
/// workspace. Hand-listed because `cargo metadata` ordering is by
/// alphabet, not by the DAG.
fn run_publish_check(sh: &Shell) -> Result<()> {
    // Same order used in `.github/workflows/release.yml` (PR #51).
    let crates = [
        "midstreamer-temporal-compare",
        "midstreamer-scheduler",
        "midstreamer-quic",
        "midstreamer-attractor",
        "midstreamer-neural-solver",
        "midstreamer-strange-loop",
        "midstream",
    ];

    for crate_name in crates {
        println!("--> cargo publish --dry-run -p {crate_name} --allow-dirty");
        cmd!(sh, "cargo publish --dry-run -p {crate_name} --allow-dirty").run()?;
    }

    println!("\npublish-check ok ({} crates)", crates.len());
    Ok(())
}

/// `wasm-pack build` for the WASM crates. Builds web / bundler /
/// nodejs targets for each. Requires `wasm-pack` on PATH (see
/// `cargo xtask install`).
fn run_wasm(sh: &Shell) -> Result<()> {
    println!("--> wasm-pack build --target web wasm-bindings");
    cmd!(sh, "wasm-pack build --target web wasm-bindings").run()?;

    println!("--> wasm-pack build --target bundler wasm-bindings");
    cmd!(sh, "wasm-pack build --target bundler wasm-bindings").run()?;

    println!("--> wasm-pack build --target nodejs wasm-bindings");
    cmd!(sh, "wasm-pack build --target nodejs wasm-bindings").run()?;

    println!("\nwasm builds ok");
    Ok(())
}

/// Install the dev-machine toolchain bits this repo needs.
///
/// Idempotent — each `cargo install` is a no-op if the binary is
/// already on PATH at a satisfying version.
fn run_install(sh: &Shell) -> Result<()> {
    println!("--> rustup component add clippy rustfmt");
    cmd!(sh, "rustup component add clippy rustfmt").run()?;

    println!("--> rustup target add wasm32-unknown-unknown");
    cmd!(sh, "rustup target add wasm32-unknown-unknown").run()?;

    println!("--> cargo install cargo-audit --locked");
    cmd!(sh, "cargo install cargo-audit --locked").run()?;

    println!("--> cargo install cargo-deny --locked");
    cmd!(sh, "cargo install cargo-deny --locked").run()?;

    println!("--> cargo install cargo-edit --locked");
    cmd!(sh, "cargo install cargo-edit --locked").run()?;

    println!("--> cargo install wasm-pack --locked");
    cmd!(sh, "cargo install wasm-pack --locked").run()?;

    println!("\ninstall ok");
    Ok(())
}
