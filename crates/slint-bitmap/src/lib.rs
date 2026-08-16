//! slint-bitmap：可复用像素风 Slint 1.17 组件库。
//!
//! 提供：
//! - Slint 组件（经 `@slint_bitmap` 导入，嵌入你自己的 `.slint`）：
//!   `PixelPainter` 画板、`PixelTitleBar` 标题栏、`PixelButton` 按钮、`PixelCheckBox` 复选、
//!   `PixelSwitch` 开关、`PixelSlider` 滑块、`PixelTextInput` 输入框、`PixelProgressBar` 进度条、
//!   `PixelBadge` 徽章、`PixelPanel` 面板、`PixelDialog` 对话框、`Swatch` 色块。
//! - 成品窗口 [`PixelPainterWindow`]（Rust 类型，直接 `::new()` 使用）。
//! - Rust 侧一键接线：`install_painter()`（画/擦、清空、导出 PNG）与
//!   `install_title_bar_controls()`（拖拽、最小化、最大化、关闭）。
//! - `library_paths()`：供下游 crate 的 build.rs 注册 `@slint_bitmap` 库导入。
//! - 画布数据：[`Canvas`]（网格级 RGBA + 放大渲染 + PNG 导出）。
//!
//! # 在其它 Slint 项目里复用
//!
//! ## 1. 添加依赖
//!
//! ```toml
//! [dependencies]
//! slint = "1.17"
//! slint-bitmap = { path = "../slint-bitmap" }
//!
//! [build-dependencies]
//! slint-build = "1.17"
//! slint-bitmap = { path = "../slint-bitmap" }
//! ```
//!
//! ## 2. `build.rs` 注册库路径并编译自己的 `.slint`
//!
//! ```ignore
//! let config = slint_build::CompilerConfiguration::new()
//!     .with_library_paths(slint_bitmap::library_paths());
//! slint_build::compile_with_config("ui/main.slint", config).unwrap();
//! ```
//!
//! ## 3. 在 `ui/main.slint` 中导入组件
//!
//! ```slint,ignore
//! import { PixelPainter, PixelTitleBar } from "@slint_bitmap";
//! ```
//!
//! ## 4. Rust 侧接线（宏把生成类型适配到本库契约，然后一键安装）
//!
//! ```ignore
//! slint::include_modules!();
//! slint_bitmap::impl_painter_ui!(MainWindow);
//! slint_bitmap::impl_title_bar_ui!(MainWindow);
//!
//! let ui = MainWindow::new().unwrap();
//! slint_bitmap::install_painter(&ui);
//! slint_bitmap::install_title_bar_controls(&ui);
//! ui.run().unwrap();
//! ```
//!
//! 也可以直接用成品窗口，连 `.slint` 都不用写：
//!
//! ```no_run
//! use slint::ComponentHandle;
//!
//! let ui = slint_bitmap::PixelPainterWindow::new().unwrap();
//! slint_bitmap::install_painter(&ui);
//! slint_bitmap::install_title_bar_controls(&ui);
//! ui.run().unwrap();
//! ```
//!
#![deny(unsafe_code)]

pub mod canvas;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

pub use canvas::{Canvas, CELL_PX, GRID};

slint::include_modules!();

/// 返回 `@slint_bitmap` 库路径表，供下游 crate 的 build.rs 注册。
///
/// 返回值是绝对路径（基于本库的 manifest 目录），与库内 build.rs 保持一致。
pub fn library_paths() -> HashMap<String, PathBuf> {
    HashMap::from([(
        "slint_bitmap".to_string(),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/lib.slint"),
    )])
}

/// 画板接线契约：任何暴露 `PixelPainter` 相同属性/回调的 Slint 组件均可实现。
///
/// 不需要手写实现——对由 `@slint_bitmap` 生成的组件调用 [`impl_painter_ui!`]
/// 即可自动适配（trait 方法名与生成代码的固有方法一致时，固有方法优先）。
pub trait PainterUi: slint::ComponentHandle {
    fn set_canvas_image(&self, image: slint::Image);
    fn set_status_text(&self, text: slint::SharedString);
    fn get_brush_size(&self) -> i32;
    fn get_eraser(&self) -> bool;
    fn get_brush_color(&self) -> slint::Color;
    fn on_paint(&self, cb: impl FnMut(f32, f32, bool) + 'static);
    fn on_clear_canvas(&self, cb: impl FnMut() + 'static);
    fn on_save_png(&self, cb: impl FnMut() + 'static);
}

/// 为生成类型实现 [`PainterUi`]。用法：
/// ```rust,ignore
/// slint_bitmap::impl_painter_ui!(MainWindow);
/// ```
#[macro_export]
macro_rules! impl_painter_ui {
    ($ty:ty) => {
        impl $crate::PainterUi for $ty {
            fn set_canvas_image(&self, image: ::slint::Image) {
                self.set_canvas_image(image);
            }
            fn set_status_text(&self, text: ::slint::SharedString) {
                self.set_status_text(text);
            }
            fn get_brush_size(&self) -> i32 {
                self.get_brush_size()
            }
            fn get_eraser(&self) -> bool {
                self.get_eraser()
            }
            fn get_brush_color(&self) -> ::slint::Color {
                self.get_brush_color()
            }
            fn on_paint(&self, cb: impl FnMut(f32, f32, bool) + 'static) {
                self.on_paint(cb);
            }
            fn on_clear_canvas(&self, cb: impl FnMut() + 'static) {
                self.on_clear_canvas(cb);
            }
            fn on_save_png(&self, cb: impl FnMut() + 'static) {
                self.on_save_png(cb);
            }
        }
    };
}

/// 像素标题栏接线契约：带 `drag-start / minimize / toggle-maximize / close-window`
/// 回调的组件（如 `PixelTitleBar` 或组装后的窗口）均可实现。
pub trait TitleBarUi: slint::ComponentHandle {
    fn on_drag_start(&self, cb: impl FnMut() + 'static);
    fn on_minimize(&self, cb: impl FnMut() + 'static);
    fn on_toggle_maximize(&self, cb: impl FnMut() + 'static);
    fn on_close_window(&self, cb: impl FnMut() + 'static);
}

/// 为生成类型实现 [`TitleBarUi`]。用法：
/// ```rust,ignore
/// slint_bitmap::impl_title_bar_ui!(MainWindow);
/// ```
#[macro_export]
macro_rules! impl_title_bar_ui {
    ($ty:ty) => {
        impl $crate::TitleBarUi for $ty {
            fn on_drag_start(&self, cb: impl FnMut() + 'static) {
                self.on_drag_start(cb);
            }
            fn on_minimize(&self, cb: impl FnMut() + 'static) {
                self.on_minimize(cb);
            }
            fn on_toggle_maximize(&self, cb: impl FnMut() + 'static) {
                self.on_toggle_maximize(cb);
            }
            fn on_close_window(&self, cb: impl FnMut() + 'static) {
                self.on_close_window(cb);
            }
        }
    };
}

// 本库自带的成品窗口直接实现接线契约，开箱即用：
// `PixelPainterWindow` 无需再调用宏，可直接传给 install_painter / install_title_bar_controls。
crate::impl_painter_ui!(PixelPainterWindow);
crate::impl_title_bar_ui!(PixelPainterWindow);

/// 安装画板接线：画/擦、清空、导出 PNG，并返回画布句柄（供高级用法直接读写）。
///
/// 导出目录为进程当前工作目录，文件名 `pixel-art-<时间戳>.png`。
pub fn install_painter<T: PainterUi + 'static>(ui: &T) -> Rc<RefCell<Canvas>> {
    let canvas = Rc::new(RefCell::new(Canvas::new(GRID, CELL_PX)));
    ui.set_canvas_image(canvas.borrow().render_display());

    // 画笔 / 擦除
    let weak = ui.as_weak();
    let canvas_paint = canvas.clone();
    ui.on_paint(move |x, y, erase| {
        let Some(ui) = weak.upgrade() else { return };
        let col = cell_index(x);
        let row = cell_index(y);
        let size = ui.get_brush_size().max(1) as usize;
        let color = if erase || ui.get_eraser() {
            None
        } else {
            Some(to_rgba(ui.get_brush_color()))
        };
        canvas_paint.borrow_mut().paint_brush(col, row, size, color);
        ui.set_canvas_image(canvas_paint.borrow().render_display());
    });

    // 清空画布
    let weak = ui.as_weak();
    let canvas_clear = canvas.clone();
    ui.on_clear_canvas(move || {
        let Some(ui) = weak.upgrade() else { return };
        canvas_clear.borrow_mut().clear();
        ui.set_canvas_image(canvas_clear.borrow().render_display());
        ui.set_status_text("画布已清空".into());
    });

    // 导出 PNG（保存到当前工作目录）
    let weak = ui.as_weak();
    let canvas_save = canvas.clone();
    ui.on_save_png(move || {
        let Some(ui) = weak.upgrade() else { return };
        let path = export_path();
        match canvas_save.borrow().export_png(&path) {
            Ok(()) => ui.set_status_text(format!("已保存: {}", path.display()).into()),
            Err(e) => ui.set_status_text(format!("保存失败: {e}").into()),
        }
    });

    canvas
}

/// 安装像素标题栏窗口控制：拖拽移动、最小化、最大化/还原、关闭。
pub fn install_title_bar_controls<T: TitleBarUi + 'static>(ui: &T) {
    use slint::winit_030::WinitWindowAccessor;

    // 拖拽移动
    let weak = ui.as_weak();
    ui.on_drag_start(move || {
        let Some(ui) = weak.upgrade() else { return };
        ui.window().with_winit_window(|w| {
            let _ = w.drag_window();
        });
    });

    // 最小化
    let weak = ui.as_weak();
    ui.on_minimize(move || {
        if let Some(ui) = weak.upgrade() {
            ui.window().set_minimized(true);
        }
    });

    // 最大化 / 还原
    let weak = ui.as_weak();
    ui.on_toggle_maximize(move || {
        if let Some(ui) = weak.upgrade() {
            let maximized = ui.window().is_maximized();
            ui.window().set_maximized(!maximized);
        }
    });

    // 关闭
    let weak = ui.as_weak();
    ui.on_close_window(move || {
        if let Some(ui) = weak.upgrade() {
            let _ = ui.window().hide();
            let _ = slint::quit_event_loop();
        }
    });
}

/// 逻辑坐标 → 网格列/行（越界裁剪到画布内）。
fn cell_index(pos: f32) -> usize {
    let cell = (pos / CELL_PX as f32) as i32;
    cell.clamp(0, GRID as i32 - 1) as usize
}

fn to_rgba(color: slint::Color) -> [u8; 4] {
    [color.red(), color.green(), color.blue(), color.alpha()]
}

fn export_path() -> PathBuf {
    let name = format!("pixel-art-{}.png", timestamp());
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(name)
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
