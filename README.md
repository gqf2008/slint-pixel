# slint-bitmap

基于 **Rust + Slint 1.17** 的可复用像素风组件库：一个 16×16 像素画板 widget +
自绘像素标题栏，并附带一个成品像素风窗口与演示程序。

- **可复用**：`PixelPainter` 画板 widget / `PixelTitleBar` 像素标题栏 / `Swatch` 色块 /
  `PixelButton` 像素按钮均可通过 `@slint_bitmap` 导入到任意 Slint 项目。
- **一键接线**：Rust 侧调用 `install_painter()` / `install_title_bar_controls()`，
  画/擦、清空、导出 PNG、窗口控制全部自动接好。
- **自绘像素标题栏**：无系统边框（`no-frame`），标题栏可拖拽移动、双击最大化，
  右侧提供 最小化 / 最大化 / 关闭 按钮。
- **像素画布**：16×16 网格，格子放大 24px 显示（棋盘格 + 网格线 + 立体像素块）。
- **PICO-8 色板**：16 色，点击选色。
- **画笔**：1×1 / 2×2 / 3×3 笔刷；左键画、右键擦除。
- **导出**：一键保存为网格分辨率 PNG（透明背景），存到当前工作目录。
- **跨平台**：仅依赖 `slint` + `image`（纯 Rust），无平台特定代码；
  窗口控制使用 Slint/Winit 跨平台 API（Windows / macOS / Linux）。

![screenshot](docs/screenshot.png)

## 运行演示

```bash
cargo run
```

## 结构（workspace）

```
crates/
├── slint-bitmap/                  # 组件库（lib）
│   ├── ui/pixel_painter_widget.slint    # 组件入口：PixelPainter / PixelTitleBar / Swatch / PixelButton
│   ├── ui/pixel_painter_window.slint    # 成品窗口 PixelPainterWindow（无边框 + 标题栏 + 画板）
│   └── src/
│       ├── lib.rs                 # library_paths()、接线 trait/宏、install_painter() 等
│       └── canvas.rs              # 画布数据、放大渲染、PNG 导出（含单元测试）
└── slint-bitmap-demo/             # 演示程序（bin，作为下游消费者组装窗口）
    ├── ui/main.slint              # 通过 @slint_bitmap 导入组件并组装
    └── src/main.rs                # 调用 install_painter / install_title_bar_controls
```

## 在其它 Slint 项目里复用

### 1. 添加依赖

```toml
[dependencies]
slint = "1.17"
slint-bitmap = { path = "path/to/slint-bitmap/crates/slint-bitmap" }

[build-dependencies]
slint-build = "1.17"
slint-bitmap = { path = "path/to/slint-bitmap/crates/slint-bitmap" }
```

### 2. 在 `build.rs` 注册 `@slint_bitmap` 库并编译自己的 UI

```rust
fn main() {
    println!("cargo:rerun-if-changed=ui/main.slint");
    let library_paths = slint_bitmap::library_paths();
    for path in library_paths.values() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let config = slint_build::CompilerConfiguration::new().with_library_paths(library_paths);
    slint_build::compile_with_config("ui/main.slint", config).unwrap();
}
```

### 3. 在 `ui/main.slint` 里导入组件并组装

```slint
import { PixelPainter, PixelTitleBar } from "@slint_bitmap";

export component MainWindow inherits Window {
    no-frame: true;
    preferred-width: 720px;
    preferred-height: 560px;

    in-out property <image> canvas-image;
    in-out property <string> status-text: "就绪";

    callback paint(length, length, bool);
    callback clear-canvas();
    callback save-png();
    callback drag-start();
    callback minimize();
    callback toggle-maximize();
    callback close-window();

    VerticalLayout {
        padding: 12px;
        spacing: 10px;
        PixelTitleBar {
            title: "我的像素窗口";
            drag-start => { root.drag-start(); }
            minimize => { root.minimize(); }
            toggle-maximize => { root.toggle-maximize(); }
            close-window => { root.close-window(); }
        }
        PixelPainter {
            canvas-image <=> root.canvas-image;
            status-text <=> root.status-text;
            paint(px, py, erase) => { root.paint(px, py, erase); }
            clear-canvas => { root.clear-canvas(); }
            save-png => { root.save-png(); }
        }
    }
}
```

完整参考：`crates/slint-bitmap-demo/ui/main.slint`。

### 4. Rust 侧一键接线

```rust
slint::include_modules!();

// 把生成的 MainWindow 适配到组件库契约（一行宏，无需手写胶水代码）
slint_bitmap::impl_painter_ui!(MainWindow);
slint_bitmap::impl_title_bar_ui!(MainWindow);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = MainWindow::new()?;
    slint_bitmap::install_painter(&ui);           // 画/擦、清空、导出 PNG
    slint_bitmap::install_title_bar_controls(&ui); // 拖拽、最小化、最大化、关闭
    ui.run()?;
    Ok(())
}
```

### 不想写 `.slint`？直接用成品窗口

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = slint_bitmap::PixelPainterWindow::new()?;
    slint_bitmap::install_painter(&ui);
    slint_bitmap::install_title_bar_controls(&ui);
    ui.run()?;
    Ok(())
}
```

### 自定义主题

`PixelPainter` 暴露了 `page / panel / panel-light / edge / shadow / text-color / dim /
highlight / danger` 与 `palette`（PICO-8 色板数组）等 `in` 属性，宿主可在 `.slint`
里直接覆盖，无需改组件库。

## 操作

| 操作 | 说明 |
| --- | --- |
| 左键拖动画布 | 用当前颜色/笔刷作画 |
| 右键拖动画布 | 擦除 |
| 点击色板 | 选择颜色 |
| 画笔 / 橡皮擦 | 切换工具 |
| 1×1 / 2×2 / 3×3 | 切换笔刷大小 |
| 清空 | 清空画布 |
| 存 PNG | 导出 `pixel-art-<时间戳>.png` |
| 标题栏拖拽 | 移动窗口 |
| 标题栏双击 | 最大化 / 还原 |
| 标题栏右侧按钮 | 最小化 / 最大化 / 关闭 |

## 说明

- Slint 在 crates.io 上最新稳定版为 1.17（无 0.17 版本线）。
- Linux Wayland 下合成器可能强制保留系统装饰，`no-frame` 效果取决于合成器。
- 高 DPI（150%）显示器下按物理像素渲染，画布仍保持像素锐利。
- `install_painter` 导出的 PNG 保存在进程当前工作目录。