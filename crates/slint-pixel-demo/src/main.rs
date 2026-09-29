#![deny(unsafe_code)]

use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use slint::winit_030::winit;
use slint::winit_030::{CustomApplicationHandler, EventResult};

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

const GALLERY_LOGICAL_WIDTH: f32 = 760.0;
const GALLERY_LOGICAL_HEIGHT: f32 = 640.0;

#[derive(Default)]
struct AppState {
    gallery: Option<GalleryWindow>,
}

/// 在 winit `resumed` 阶段先把主窗口目标位置算好，再创建 Slint 窗口。
/// 这样窗口属性钩子拿到位置后，Slint 会在原生窗口创建前完成定位，不会先显示再跳动。
struct MainWindowPositioner {
    state: Rc<RefCell<AppState>>,
    initial_center: Rc<RefCell<Option<winit::dpi::PhysicalPosition<i32>>>>,
}

impl CustomApplicationHandler for MainWindowPositioner {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) -> EventResult {
        if self.state.borrow().gallery.is_some() {
            return EventResult::Propagate;
        }

        if let Some(monitor) = event_loop.primary_monitor() {
            let scale = monitor.scale_factor() as f32;
            let window_width = (GALLERY_LOGICAL_WIDTH * scale).round() as i32;
            let window_height = (GALLERY_LOGICAL_HEIGHT * scale).round() as i32;

            let monitor_position = monitor.position();
            let monitor_size = monitor.size();
            let x = monitor_position.x + (monitor_size.width as i32 - window_width) / 2;
            let y = monitor_position.y + (monitor_size.height as i32 - window_height) / 2;

            *self.initial_center.borrow_mut() = Some(winit::dpi::PhysicalPosition::new(x, y));
        }

        match setup_gallery() {
            Ok(gallery) => {
                self.state.borrow_mut().gallery = Some(gallery);
                // 只让主窗口使用这个初始位置；后续画板 / 主题编辑器窗口按各自逻辑定位。
                *self.initial_center.borrow_mut() = None;
            }
            Err(err) => {
                eprintln!("failed to create gallery: {err}");
                event_loop.exit();
            }
        }

        EventResult::Propagate
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let state = Rc::new(RefCell::new(AppState::default()));
    let initial_center = Rc::new(RefCell::new(None));

    let initial_center_for_hook = initial_center.clone();
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .with_winit_window_attributes_hook(move |attributes| {
            // 圆角窗身（PixelWindowBody + PixelTheme.window-radius）要求窗口本身透明
            let attributes = slint_pixel::transparent_window(attributes);
            if let Some(position) = *initial_center_for_hook.borrow() {
                attributes
                    .with_inner_size(winit::dpi::LogicalSize::new(
                        GALLERY_LOGICAL_WIDTH,
                        GALLERY_LOGICAL_HEIGHT,
                    ))
                    .with_position(position)
            } else {
                attributes
            }
        })
        .with_winit_custom_application_handler(MainWindowPositioner {
            state: state.clone(),
            initial_center: initial_center.clone(),
        })
        .select()?;

    slint::run_event_loop()?;
    Ok(())
}

fn setup_gallery() -> Result<GalleryWindow, Box<dyn Error>> {
    use slint::ComponentHandle;

    let gallery = GalleryWindow::new()?;
    // 在 show 前显式锁定主窗口尺寸，避免 Slint 在原生窗口创建后再异步调整尺寸，
    // 从而保证窗口属性钩子里设置的初始位置不会被 resize 过程改变。
    gallery
        .window()
        .set_size(slint::WindowSize::Logical(slint::LogicalSize::new(
            GALLERY_LOGICAL_WIDTH,
            GALLERY_LOGICAL_HEIGHT,
        )));
    slint_pixel::install_title_bar_controls(&gallery);
    slint_pixel::install_window_resize(&gallery);

    // 可选：命令行选初始预设 —— `cargo run -- soft` / `dark`（默认 classic 经典像素风）
    if let Some(preset) = std::env::args().nth(1) {
        gallery.set_theme_preset(preset.as_str().into());
    }

    // 画廊里的“打开像素画板”：新开一个画板窗口并保持存活
    let painters: Rc<RefCell<Vec<MainWindow>>> = Rc::new(RefCell::new(Vec::new()));
    let painters_open = painters.clone();
    let attach_timers: Rc<RefCell<Vec<slint::Timer>>> = Rc::new(RefCell::new(Vec::new()));
    let attach_timers_p = attach_timers.clone();
    let gallery_ref = gallery.clone_strong();
    gallery.on_open_painter(move || {
        if let Ok(painter) = MainWindow::new() {
            // Slint 的 global 不跨窗口共享：把画廊当前预设带给新窗口
            painter.set_theme_preset(gallery_ref.get_theme_preset());
            slint_pixel::install_painter(&painter);
            slint_pixel::install_title_bar_controls_no_quit(&painter);
            slint_pixel::install_window_resize(&painter);
            place_window_before_show(&painter, &gallery_ref, 720.0, 560.0);

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
        let code = theme_code(
            "slint-pixel 主题覆盖（粘贴到你的 .slint，或设回 PixelTheme）",
            &[
                ("bg", hex(g.get_t_bg())),
                ("panel", hex(g.get_t_panel())),
                ("hover", hex(g.get_t_hover())),
                ("edge", hex(g.get_t_edge())),
                ("shadow", hex(g.get_t_shadow())),
                ("text", hex(g.get_t_text())),
                ("dim", hex(g.get_t_dim())),
                ("accent", hex(g.get_t_accent())),
                ("danger", hex(g.get_t_danger())),
                ("success", hex(g.get_t_success())),
                ("warning", hex(g.get_t_warning())),
                ("info", hex(g.get_t_info())),
                ("on-accent", hex(g.get_t_on_accent())),
                ("primary-face", hex(g.get_t_primary_face())),
                ("primary-text", hex(g.get_t_primary_text())),
                ("radius", px(g.get_t_radius())),
                ("radius-sm", px(g.get_t_radius_sm())),
                ("window-radius", px(g.get_t_window_radius())),
            ],
        );
        g.set_generated_theme(code.into());
    });

    // 主题编辑器：打开独立窗口（实时改 PixelTheme，画廊/画板同步）
    let editors: Rc<RefCell<Vec<ThemeEditorWindow>>> = Rc::new(RefCell::new(Vec::new()));
    let editors_open = editors.clone();
    let gallery_ref2 = gallery.clone_strong();
    gallery.on_open_theme_editor(move || {
        if let Ok(editor) = ThemeEditorWindow::new() {
            editor.set_theme_preset(gallery_ref2.get_theme_preset());
            slint_pixel::install_title_bar_controls_no_quit(&editor);
            slint_pixel::install_window_resize(&editor);
            wire_generate_theme(&editor);
            place_window_before_show(&editor, &gallery_ref2, 620.0, 780.0);

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
    Ok(gallery)
}

fn place_window_before_show<C: slint::ComponentHandle, O: slint::ComponentHandle>(
    child: &C,
    owner: &O,
    logical_width: f32,
    logical_height: f32,
) {
    let scale = owner.window().scale_factor();
    let owner_pos = owner.window().position();
    let owner_size = owner.window().size();

    let child_width = (logical_width * scale) as i32;
    let child_height = (logical_height * scale) as i32;
    let x = owner_pos.x + (owner_size.width as i32 - child_width) / 2;
    let y = owner_pos.y + (owner_size.height as i32 - child_height) / 2;

    child.window().set_position(slint::WindowPosition::Physical(
        slint::PhysicalPosition::new(x, y),
    ));
}

fn wire_generate_theme(editor: &ThemeEditorWindow) {
    let weak = editor.as_weak();
    editor.on_generate_theme(move || {
        let Some(ui) = weak.upgrade() else { return };
        let code = theme_code(
            "slint-pixel 主题覆盖",
            &[
                ("bg", hex(ui.get_t_bg())),
                ("panel", hex(ui.get_t_panel())),
                ("hover", hex(ui.get_t_hover())),
                ("edge", hex(ui.get_t_edge())),
                ("shadow", hex(ui.get_t_shadow())),
                ("text", hex(ui.get_t_text())),
                ("dim", hex(ui.get_t_dim())),
                ("accent", hex(ui.get_t_accent())),
                ("danger", hex(ui.get_t_danger())),
                ("success", hex(ui.get_t_success())),
                ("warning", hex(ui.get_t_warning())),
                ("info", hex(ui.get_t_info())),
                ("on-accent", hex(ui.get_t_on_accent())),
                ("primary-face", hex(ui.get_t_primary_face())),
                ("primary-text", hex(ui.get_t_primary_text())),
                ("radius", px(ui.get_t_radius())),
                ("radius-sm", px(ui.get_t_radius_sm())),
                ("window-radius", px(ui.get_t_window_radius())),
            ],
        );
        ui.set_generated_theme(code.into());
    });
}

/// 生成 PixelTheme 覆盖代码；画廊与主题编辑器共用一份，避免两处漂移。
fn theme_code(comment: &str, values: &[(&str, String)]) -> String {
    let mut code = format!("// {comment}\nimport {{ PixelTheme }} from \"@slint_pixel\";\n\n");
    for (name, value) in values {
        code.push_str(&format!("PixelTheme.{name} = {value};\n"));
    }
    code
}

fn px(len: f32) -> String {
    format!("{len:.0}px")
}

fn hex(c: slint::Color) -> String {
    format!("#{:02X}{:02X}{:02X}", c.red(), c.green(), c.blue())
}
