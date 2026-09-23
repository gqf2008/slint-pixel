//! 打包合规守卫：`.crate` 必须自带 LICENSE 全文。
//!
//! 背景（2026-09-23 核查）：crates.io 上 0.2.1 / 0.2.2 / 0.2.3 的包内**都没有 LICENSE** ——
//! 全文一直放在 workspace 根（`/LICENSE`），而 cargo 只自动收**包目录内**的 `LICENSE*`，
//! workspace 根那份不会进 `.crate`。MIT 要求"许可与版权声明随副本分发"，所以把全文放进
//! `crates/slint-pixel/LICENSE`，并用这条测试钉住：谁把副本删掉/挪走，这里就红。
//!
//! 只依赖包内文件（不读 workspace 根的 LICENSE）：这样从 crates.io 下载的 `.crate`
//! 里跑 `cargo test` 也是同样的断言，不会出现"只有开发树里能过"。

use std::path::PathBuf;

#[test]
fn crate_ships_full_license_text() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("LICENSE");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "缺少 {}：{e}\n\
             cargo 只自动收**包目录内**的 LICENSE*，workspace 根的不会进 .crate —— \
             发布前这份副本必须存在（见本文件头说明）。",
            path.display()
        )
    });

    assert!(
        text.starts_with("MIT License"),
        "LICENSE 全文应以 `MIT License` 开头，实际开头：{:?}",
        text.chars().take(40).collect::<String>()
    );
    assert!(
        text.contains("Permission is hereby granted, free of charge"),
        "LICENSE 应包含 MIT 许可正文（不是只写了名字的空壳）"
    );
    assert!(text.contains("gqf2008"), "LICENSE 应包含版权署名行");
}
