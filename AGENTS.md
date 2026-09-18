# Repository Guidelines

`slint-pixel` is a Rust + Slint 1.17 reusable pixel-art component library with a demo consumer.

## 协同拓扑（walgit 主仓 + GitHub 镜像）

- **主仓 / 权威事实源**：walgit `http://127.0.0.1:8081/gqf2008/slint-pixel.git`，remote 名 `origin`。日常分支、提交、合并、tag 一律推 `origin`。
- **镜像 + 发布**：GitHub `https://github.com/gqf2008/slint-pixel.git`，remote 名 `github`，只读镜像；由 `~/.walgit/sync-to-github.sh` 的常驻循环（screen `walgit-sync-github`，60s）自动同步 heads+tags。不要手推 GitHub，不要双推（双事实源会漂移）。
- **协作记账**：issue / PR / 评审 / 合并 / CI 都是 `refs/collab/*` 上的签名条目，用
  `walgit --config ~/.walgit/walgit.toml collab <ls|thread|pr|board|report>` 读、
  `walgit --config ~/.walgit/walgit.toml collab entry ...` 写（key 传 `~/.walgit/keys/<principal>.ed25519` **路径**）。
  只开分支不记账 = 没有协作记录。
- **去中心化 CI**：任务随代码走，声明在 `.walgit/ci.toml`（被测提交里的那一版才算数）。runner 为每个任务在
  `$TMPDIR` 下建临时 worktree 执行，所以**必须把 `TMPDIR` 与 `CARGO_TARGET_DIR` 钉到数据卷**，否则检出与
  cargo 产物会落 228 GiB 内盘（链接期 `errno 28`）：

  ```sh
  TMPDIR=/Volumes/DataExt/tmp \
  CARGO_TARGET_DIR=/Volumes/DataExt/tmp/slint-pixel-ci-target \
    walgit ci run --repo /Volumes/DataExt/GitHub/slint-pixel --remote origin \
      --actor ci-runner --key ~/.walgit/keys/ci-runner.ed25519
  ```

  加 `--once` 跑单轮。结果用 `walgit ci status` / `collab board` 查看；看板泳道由 `.walgit/board.toml` 声明。
  `--key` 传密钥**路径**，不要传密钥内容。
- **发版流程**：`cargo publish`（crates.io）→ 打 tag → `git push origin vX.Y.Z` → 镜像循环同步 GitHub →
  `gh release create vX.Y.Z --generate-notes`（`gh release create` 是唯一一次对 GitHub 的写操作，镜像本身只读）。
  发布前本地门禁必须全绿。
- **worktree**：开发用 `<repo>/.worktrees/<name>`（已 gitignore），合并后当轮清理。

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

GitHub Actions 已不用于日常门禁；fmt / clippy / test 由 walgit 去中心化 CI（`.walgit/ci.toml`）执行，
提交前在本地跑同一组命令（注意 `CARGO_TARGET_DIR` 指到数据卷 `/Volumes/DataExt/tmp`）。

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
- Open PRs against `master` on the walgit remote, link the collab issue thread (`cc-ai-*`), and include a summary, verification commands, and screenshots for UI changes.

## Security & Configuration Tips
- Do not commit `target/` or exported `pixel-art-*.png`; both are gitignored.
- Do not add secrets or machine-specific configuration.
