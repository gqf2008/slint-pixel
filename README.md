# slint-bitmap

基于 **Rust + Slint 1.17** 的像素风可复用组件库：16×16 像素画板 widget、
自绘像素标题栏，以及一套常用像素风控件；附带组件画廊与画板演示程序。

- **可复用**：所有组件通过 `@slint_bitmap` 导入到任意 Slint 项目（见下方组件清单）。
- **一键接线**：Rust 侧调用 `install_painter()` / `install_title_bar_controls()`，
  画/擦、清空、导出 PNG、窗口控制全部自动接好。
- **自绘像素标题栏**：无系统边框（`no-frame`），可拖拽、双击最大化，右侧最小化/最大化/关闭。
- **像素画布**：16×16 网格，格子放大 24px 显示（棋盘格 + 网格线 + 立体像素块）。
- **PICO-8 色板**：16 色，点击选色；画笔 1×1 / 2×2 / 3×3；左键画、右键擦除。
- **导出**：一键保存为网格分辨率 PNG（透明背景），存到当前工作目录。
- **跨平台**：仅依赖 `slint` + `image`（纯 Rust），无平台特定代码。

![gallery](docs/gallery.png)

## 组件清单

所有组件都是 `in`/`in-out` 属性 + 回调，宿主可覆盖主题色，无系统依赖、跨平台。

| 组件 | 说明 |
| --- | --- |
| `PixelPainter` | 16×16 像素画板 widget（画布 + 色板 + 工具栏 + 状态栏） |
| `PixelTitleBar` | 可拖拽像素标题栏（拖拽/最小化/最大化/关闭回调） |
| `PixelButton` | 像素按压按钮（按下下沉效果） |
| `PixelCheckBox` | 复选框（像素勾选 + 文字标签） |
| `PixelSwitch` | 开关（滑块滑动动画） |
| `PixelSlider` | 滑块（拖拽取值，min/max，`changed(value)` 回调） |
| `PixelTextInput` | 单行文本输入（占位符 + 焦点高亮 + `accepted`/`edited`） |
| `PixelProgressBar` | 进度条（0..1，可选百分比文字） |
| `PixelBadge` | 徽章/标签 |
| `PixelPanel` | 分组面板（标题 + 内容插槽 `@children`） |
| `PixelDialog` | 模态对话框（遮罩 + 标题 + 内容插槽 + 确定/取消） |
| `Swatch` | 像素风色块 |

> 提示：提示气泡请直接用 Slint 内置 `Tooltip`（延迟出现、自动跟随指针）：
> ```slint
> PixelButton {
>     text: "悬停";
>     Tooltip { text: @markdown("**提示文字**"); }
> }
> ```
> 在盒布局中放置组件时，未显式给 `width` 的组件会拉伸占满剩余空间，
> 需要固定宽度的场景请显式指定（如 `PixelButton { width: 90px; }`）。

## 运行演示

```bash
cargo run
```

启动后打开 **组件画廊**（展示全部常用控件），点画廊里的 **打开像素画板 →** 可打开画板窗口。

## 结构（workspace）

```
crates/
├── slint-bitmap/                  # 组件库（lib）
│   ├── ui/lib.slint               # @slint_bitmap 汇总入口（re-export 全部组件）
│   ├── ui/pixel_painter_widget.slint   # 画板 + 标题栏 + 按钮 + 色块
│   ├── ui/pixel_widgets.slint          # 常用控件：复选/开关/滑块/输入/进度/徽章/面板/对话框
│   ├── ui/pixel_painter_window.slint   # 成品窗口 PixelPainterWindow
│   └── src/
│       ├── lib.rs                 # library_paths()、接线 trait/宏、install_painter() 等
│       └── canvas.rs              # 画布数据、放大渲染、PNG 导出（含单元测试）
└── slint-bitmap-demo/             # 演示程序（bin，作为下游消费者组装窗口）
    ├── ui/gallery.slint           # 组件画廊窗口
    ├── ui/main.slint              # 画板窗口（通过 @slint_bitmap 组装）
    └── src/main.rs                # 接线：install_painter / install_title_bar_controls / 打开画板
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

完整参考：`crates/slint-bitmap-demo/ui/main.slint` 与 `crates/slint-bitmap-demo/ui/gallery.slint`。

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

所有组件暴露 `in` 主题属性（如 `PixelPainter` 的 `page / panel / panel-light / edge /
shadow / text-color / dim / highlight / danger` 与 `palette`；`PixelSlider` 的
`track / fill / thumb / border / shadow` 等），宿主可在 `.slint` 里直接覆盖。

## 操作（画板）

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
- 暂未内置：下拉选择框 / 单选框组 / 菜单 / 表格 / 滚动条皮肤等，可按需在
  `pixel_widgets.slint` 里扩展（组件均遵循同一像素风主题约定）。