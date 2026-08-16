#![deny(unsafe_code)]

use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

// build.rs 编译了两个入口（ui/main.slint 画板窗口、ui/gallery.slint 组件画廊），
// `slint::include_modules!()` 只会包含最后一个，这里手动包含两个生成文件。
include!(concat!(env!("OUT_DIR"), "/main.rs"));
include!(concat!(env!("OUT_DIR"), "/gallery.rs"));
include!(concat!(env!("OUT_DIR"), "/theme_editor.rs"));

// 把生成的窗口类型适配到组件库接线契约（固有方法优先于 trait 方法）
slint_pixel::impl_painter_ui!(MainWindow);
slint_pixel::impl_title_bar_ui!(MainWindow);
slint_pixel::impl_resize_ui!(MainWindow);
slint_pixel::impl_title_bar_ui!(GalleryWindow);
slint_pixel::impl_resize_ui!(GalleryWindow);
slint_pixel::impl_title_bar_ui!(ThemeEditorWindow);
slint_pixel::impl_resize_ui!(ThemeEditorWindow);

fn main() -> Result<(), Box<dyn Error>> {
    use slint::ComponentHandle;

    let gallery = GalleryWindow::new()?;
    slint_pixel::install_title_bar_controls(&gallery);
    slint_pixel::install_window_resize(&gallery);

    // 画廊里的“打开像素画板”：新开一个画板窗口并保持存活
    let painters: Rc<RefCell<Vec<MainWindow>>> = Rc::new(RefCell::new(Vec::new()));

    let painters_open = painters.clone();
    let attach_timers: Rc<RefCell<Vec<slint::Timer>>> = Rc::new(RefCell::new(Vec::new()));
    let attach_timers_p = attach_timers.clone();
    let gallery_ref = gallery.clone_strong();
    gallery.on_open_painter(move || {
        if let Ok(painter) = MainWindow::new() {
            slint_pixel::install_painter(&painter);
            slint_pixel::install_title_bar_controls_no_quit(&painter);
            slint_pixel::install_window_resize(&painter);
            if painter.show().is_ok() {
                // 挂到画廊窗口，任务栏不单独显示（show 后 winit 窗口异步创建，延迟再挂）
                let p2 = painter.clone_strong();
                let g2 = gallery_ref.clone_strong();
                let t = slint::Timer::default();
                t.start(
                    slint::TimerMode::SingleShot,
                    std::time::Duration::from_millis(500),
                    move || slint_pixel::attach_owner(&p2, &g2),
                );
                attach_timers_p.borrow_mut().push(t);
                painters_open.borrow_mut().push(painter);
            }
        }
    });

    // 主题编辑器：读 PixelTheme 生成 .slint 覆盖代码
    let weak = gallery.as_weak();
    gallery.on_generate_theme(move || {
        let Some(g) = weak.upgrade() else { return };
        let code = format!(
            "// slint-pixel 主题覆盖（粘贴到你的 .slint，或设回 PixelTheme）\nimport {{ PixelTheme }} from \"@slint_pixel\";\n\nPixelTheme.bg = {};\nPixelTheme.panel = {};\nPixelTheme.hover = {};\nPixelTheme.edge = {};\nPixelTheme.shadow = {};\nPixelTheme.text = {};\nPixelTheme.dim = {};\nPixelTheme.accent = {};\nPixelTheme.danger = {};",
            hex(g.get_t_bg()),
            hex(g.get_t_panel()),
            hex(g.get_t_hover()),
            hex(g.get_t_edge()),
            hex(g.get_t_shadow()),
            hex(g.get_t_text()),
            hex(g.get_t_dim()),
            hex(g.get_t_accent()),
            hex(g.get_t_danger()),
        );
        g.set_generated_theme(code.into());
    });

    // 主题编辑器：打开独立窗口（实时改 PixelTheme，画廊/画板同步）
    let editors: Rc<RefCell<Vec<ThemeEditorWindow>>> = Rc::new(RefCell::new(Vec::new()));
    let editors_open = editors.clone();
    let gallery_ref2 = gallery.clone_strong();
    gallery.on_open_theme_editor(move || {
        if let Ok(editor) = ThemeEditorWindow::new() {
            slint_pixel::install_title_bar_controls_no_quit(&editor);
            slint_pixel::install_window_resize(&editor);
            wire_generate_theme(&editor);
            if editor.show().is_ok() {
                // 挂到画廊窗口，任务栏不单独显示（show 后 winit 窗口异步创建，延迟再挂）
                let e2 = editor.clone_strong();
                let g2 = gallery_ref2.clone_strong();
                let t = slint::Timer::default();
                t.start(
                    slint::TimerMode::SingleShot,
                    std::time::Duration::from_millis(500),
                    move || slint_pixel::attach_owner(&e2, &g2),
                );
                attach_timers.borrow_mut().push(t);
                editors_open.borrow_mut().push(editor);
            }
        }
    });

    gallery.show()?;
    slint::run_event_loop()?;
    Ok(())
}

fn wire_generate_theme(editor: &ThemeEditorWindow) {
    let weak = editor.as_weak();
    editor.on_generate_theme(move || {
        let Some(ui) = weak.upgrade() else { return };
        let code = format!(
            "// slint-pixel 主题覆盖\nimport {{ PixelTheme }} from \"@slint_pixel\";\n\nPixelTheme.bg = {};\nPixelTheme.panel = {};\nPixelTheme.hover = {};\nPixelTheme.edge = {};\nPixelTheme.shadow = {};\nPixelTheme.text = {};\nPixelTheme.dim = {};\nPixelTheme.accent = {};\nPixelTheme.danger = {};",
            hex(ui.get_t_bg()),
            hex(ui.get_t_panel()),
            hex(ui.get_t_hover()),
            hex(ui.get_t_edge()),
            hex(ui.get_t_shadow()),
            hex(ui.get_t_text()),
            hex(ui.get_t_dim()),
            hex(ui.get_t_accent()),
            hex(ui.get_t_danger()),
        );
        ui.set_generated_theme(code.into());
    });
}

fn hex(c: slint::Color) -> String {
    format!("#{:02X}{:02X}{:02X}", c.red(), c.green(), c.blue())
}
