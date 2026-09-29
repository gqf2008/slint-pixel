//! 圆角主题的两条守卫：
//!
//! 1. **行为守卫**（`radius_actually_rounds_corners`）：把同一场景渲染两次（直角 / 圆角 12px），
//!    断言三块被测面的左上角像素分别落在「面自身颜色」和「背景红」上。圆角必须真的改变像素 ——
//!    只断言「属性赋值成功」是空转（`PixelTheme.radius` 写进去但组件没读，测试照样绿）。
//! 2. **覆盖守卫**（`every_bordered_surface_follows_theme_radius`）：`ui/*.slint` 里每个
//!    「实心背景 + 描边」的表面都必须接 `border-radius: PixelTheme.radius*`，防止新增组件漏接。
//!    透明的图标字形框 / 裁剪选框不算表面，显式豁免（它们的 `background` 是 `transparent`）。
//!
//! 渲染走 Slint 内置软件渲染器（无显示器、无 GPU），所以 CI 也能跑。

use std::cell::RefCell;
use std::rc::Rc;

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
};
use slint::platform::{Platform, PlatformError, WindowAdapter};
use slint::{ComponentHandle as _, LogicalSize};

include!(concat!(env!("OUT_DIR"), "/theme_radius.rs"));

const WIDTH: u32 = 200;
const HEIGHT: u32 = 80;
/// 探针里三块被测面的左上角坐标 + 名字（与 tests/theme_radius.slint 的固定布局一致）。
const PROBES: [(u32, u32, &str); 3] = [
    (10, 10, "PixelPanel"),
    (70, 10, "PixelCard"),
    (130, 10, "PixelBadge"),
];
/// 窗口底色，探针的「背景」也是断言基准色。
const BACKDROP: [u8; 3] = [0xff, 0x00, 0x00];

/// 无头平台：所有组件都挂到同一个 `MinimalSoftwareWindow` 上，用软件渲染器出像素。
struct SoftPlatform(Rc<MinimalSoftwareWindow>);

impl Platform for SoftPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.0.clone())
    }
}

/// `set_platform` 每个进程只能调一次；窗口本身不是 `Send`，所以用 thread-local 持有。
/// 本文件里只有渲染那条测试会走到这里，不存在两个线程抢 `set_platform` 的情况。
fn minimal_window() -> Rc<MinimalSoftwareWindow> {
    thread_local! {
        static WINDOW: RefCell<Option<Rc<MinimalSoftwareWindow>>> = const { RefCell::new(None) };
    }
    WINDOW.with(|slot| {
        let mut slot = slot.borrow_mut();
        slot.get_or_insert_with(|| {
            let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
            slint::platform::set_platform(Box::new(SoftPlatform(window.clone())))
                .expect("设置软件渲染平台（每进程只允许一次）");
            window
        })
        .clone()
    })
}

fn render(ui: &RadiusProbe) -> Vec<[u8; 4]> {
    let window = minimal_window();
    ui.window()
        .set_size(LogicalSize::new(WIDTH as f32, HEIGHT as f32));
    slint::platform::update_timers_and_animations();
    window.request_redraw();
    // 软件渲染器的目标像素类型：`PremultipliedRgbaColor` 是它有实现且字段公开的那一个。
    let mut buffer = vec![PremultipliedRgbaColor::default(); (WIDTH * HEIGHT) as usize];
    let redrawn = window.draw_if_needed(|renderer| {
        renderer.render(buffer.as_mut_slice(), WIDTH as usize);
    });
    assert!(
        redrawn,
        "软件渲染应当真的画了一帧（needs_redraw 没被置位？）"
    );
    buffer
        .iter()
        .map(|p| [p.red, p.green, p.blue, p.alpha])
        .collect()
}

fn pixel_at(frame: &[[u8; 4]], x: u32, y: u32) -> [u8; 4] {
    frame[(y * WIDTH + x) as usize]
}

#[test]
fn radius_actually_rounds_corners() {
    // 必须先装好软件渲染平台：`RadiusProbe::new()` 会去要一个窗口 adapter，
    // 没装平台时 Slint 会去初始化默认（winit）后端，而它要求事件循环在主线程。
    let _ = minimal_window();
    let ui = RadiusProbe::new().expect("构造圆角探针窗口");

    // ① 直角（库默认）：三块面的左上角都被面自己覆盖（描边或硬阴影层），不是红底
    ui.set_probe_radius(0.0);
    let square = render(&ui);
    for (x, y, name) in PROBES {
        assert_ne!(
            pixel_at(&square, x, y)[..3],
            BACKDROP,
            "{name} 在 radius=0 时左上角应被面自身覆盖，实际露出了背景色（说明布局坐标与探针不一致）"
        );
    }

    // ② 圆角 12px / 小件 6px：同一个左上角像素必须被切掉，露出红底
    ui.set_probe_radius(12.0);
    let rounded = render(&ui);
    assert_ne!(
        square, rounded,
        "改 PixelTheme.radius 后渲染结果没变化：组件没读主题圆角"
    );
    for (x, y, name) in PROBES {
        assert_eq!(
            pixel_at(&rounded, x, y)[..3],
            BACKDROP,
            "{name} 在 radius>0 时左上角应被圆角切掉（露出背景），实际仍是面自身颜色"
        );
    }

    // ③ 改回直角应当复原 —— 排除「一次性、不可逆」的假通过
    ui.set_probe_radius(0.0);
    assert_eq!(
        render(&ui),
        square,
        "把 radius 设回 0px 后应回到直角渲染结果"
    );
}

#[test]
fn every_bordered_surface_follows_theme_radius() {
    let ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    let mut missing = Vec::new();
    let mut checked = 0usize;

    for entry in std::fs::read_dir(&ui_dir).expect("读取 ui 目录") {
        let path = entry.expect("目录项").path();
        if path.extension().and_then(|e| e.to_str()) != Some("slint")
            || path.file_name().and_then(|n| n.to_str()) == Some("pixel_theme.slint")
        {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("读取 .slint 源文件");
        let lines: Vec<&str> = source.lines().collect();
        let blocks = enclosing_block(&lines);
        let name = path.file_name().unwrap().to_string_lossy().into_owned();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("border-width:") {
                continue;
            }
            let block = blocks[i];
            let indent = line.len() - trimmed.len();
            // 透明背景的元素是图标字形框 / 裁剪选框，不是可见表面
            let transparent = lines
                .iter()
                .enumerate()
                .filter(|(j, _)| blocks[*j] == block)
                .any(|(_, l)| l.trim() == "background: transparent;");
            if transparent {
                continue;
            }
            checked += 1;
            let rounded = lines.iter().enumerate().any(|(j, l)| {
                blocks[j] == block
                    && l.len() - l.trim_start().len() == indent
                    && l.trim_start().starts_with("border-radius:")
            });
            if !rounded {
                missing.push(format!("{name}:{} {}", i + 1, trimmed));
            }
        }
    }

    assert!(
        checked > 100,
        "覆盖面异常：只检查到 {checked} 个描边面，守卫可能失效了"
    );
    assert!(
        missing.is_empty(),
        "以下描边表面没有接主题圆角（新增组件时容易漏）：\n{}",
        missing.join("\n")
    );
    assert!(
        std::fs::read_to_string(ui_dir.join("pixel_theme.slint"))
            .expect("读取 pixel_theme.slint")
            .contains("in-out property <length> radius: 0px;"),
        "PixelTheme.radius 的默认值必须是 0px（既有宿主升级零外观变化）"
    );
}

/// 给每一行标注它所属元素的起始行号（取当前最内层未闭合的 `{` 所在行）。
fn enclosing_block(lines: &[&str]) -> Vec<usize> {
    let mut stack: Vec<usize> = Vec::new();
    let mut out = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        out.push(*stack.last().unwrap_or(&usize::MAX));
        for ch in line.chars() {
            match ch {
                '{' => stack.push(i),
                '}' => {
                    assert!(stack.pop().is_some(), "第 {} 行括号不配对", i + 1);
                }
                _ => {}
            }
        }
    }
    out
}
