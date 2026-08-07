//! RustMadoka desktop entry (Windows first).
//!
//! Implementation: crate `rustmadoka_app` (`src/lib.rs`).
//! Docs: `docs/PLAN_RUSTMADOKA_FULL_REWRITE.md` · `docs/tech/INSTANCE_AND_CLI.md` · `docs/HANDOFF.md`

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustmadoka_app::cli_main().await
}
