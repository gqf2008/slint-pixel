#![deny(unsafe_code)]

mod canvas;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use canvas::{Canvas, CELL_PX, GRID};
use slint::winit_030::WinitWindowAccessor;
use slint::Color;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = PixelPainter::new()?;

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

    // ===== 自绘标题栏：拖拽移动 =====
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

    ui.run()?;
    Ok(())
}

/// 逻辑坐标 → 网格列/行（越界裁剪到画布内）。
fn cell_index(pos: f32) -> usize {
    let cell = (pos / CELL_PX as f32) as i32;
    cell.clamp(0, GRID as i32 - 1) as usize
}

fn to_rgba(color: Color) -> [u8; 4] {
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
