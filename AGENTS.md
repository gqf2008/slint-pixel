# Repository Guidelines

`slint-pixel` is a Rust + Slint 1.17 reusable pixel-art component library with a demo consumer.

## Project Structure & Module Organization
- `crates/slint-pixel/` — reusable library. `src/lib.rs` exposes wiring macros and `install_*` helpers; `src/canvas.rs` owns canvas data, rendering, PNG export, and unit tests; `ui/*.slint` holds the Slint components (`lib.slint` is the `@slint_pixel` entry point); `build.rs` compiles the UI and registers the library path.
- `crates/slint-pixel-demo/` — binary demo and gallery. `ui/*.slint` assembles windows; `src/main.rs` wires the components.
- `docs/` — screenshots and assets. Root `Cargo.toml` defines the workspace.

## Build, Test, and Development Commands
- `cargo run` — run the default demo (`slint-pixel-demo`).
- `cargo build --workspace` — build both crates.
- `cargo test --workspace` — run all tests. Root `cargo test` only targets default-members, so use `--workspace`.
- `cargo test -p slint-pixel` — run library unit tests only.
- `cargo fmt --all -- --check` / `cargo fmt --all` — check or apply formatting.
- `cargo clippy --workspace --all-targets -- -D warnings` — lint gate.

No CI workflow is configured; run the fmt, clippy, and test commands locally before submitting changes.

## Coding Style & Naming Conventions
- Rust 2021 edition, standard 4-space indentation via `cargo fmt`; there is no custom `rustfmt.toml`.
- Exported Slint components use `Pixel*` PascalCase; properties and callbacks use snake_case.
- Group `.slint` files by category (`pixel_widgets`, `pixel_complex`, `pixel_extra`, `pixel_tailwind`). Add new components in the matching file, or create a new `pixel_*.slint` and import it from `ui/lib.slint`.
- `unsafe` is denied at the workspace and crate level. Platform-specific Win32 interop uses `#[allow(unsafe_code)]` with `// SAFETY:` comments.

## Testing Guidelines
- Use Rust's built-in `#[test]`; tests currently live in `crates/slint-pixel/src/canvas.rs`.
- Name tests as snake_case behavior descriptions, such as `paint_sets_cell` or `export_png_roundtrip`.
- Add regression tests for fixes and new `Canvas`/export behavior.

## Commit & Pull Request Guidelines
- Use Conventional Commits: `feat(scope):`, `fix(scope):`, `docs(scope):`, `refactor(scope):`, `style(scope):`, `test(scope):`, `perf(scope):`, `chore(scope):`. Keep one logical change per commit.
- Open PRs against `master`, link the issue, and include a summary, verification commands, and screenshots for UI changes.

## Security & Configuration Tips
- Do not commit `target/` or exported `pixel-art-*.png`; both are gitignored.
- Do not add secrets or machine-specific configuration.
