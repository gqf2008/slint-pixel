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

    let _ = child.window().set_position(slint::WindowPosition::Physical(
        slint::PhysicalPosition::new(x, y),
    ));
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
