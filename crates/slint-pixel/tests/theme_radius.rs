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
use slint::{ComponentHandle, LogicalSize};

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

/// 无头平台：每个窗口组件配一个独立的 `MinimalSoftwareWindow`（探针有多个窗口组件）。
struct SoftPlatform {
    /// 已建好、等 `create_window_adapter` 领走的窗口（`new_window()` 每次压一个）
    pending: RefCell<Vec<Rc<MinimalSoftwareWindow>>>,
}

impl SoftPlatform {
    fn new_window(&self) -> Rc<MinimalSoftwareWindow> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        self.pending.borrow_mut().push(window.clone());
        window
    }
}

impl Platform for SoftPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        self.pending
            .borrow_mut()
            .pop()
            .map(|w| w as Rc<dyn WindowAdapter>)
            .ok_or_else(|| PlatformError::Other("先 new_window() 再建窗口组件".into()))
    }
}

/// `set_platform` 每进程只能调一次；窗口不是 `Send`，所以平台用 thread-local 持有。
/// 本文件只有渲染那条测试会走到这里，不存在两个线程抢 `set_platform`。
fn platform() -> Rc<SoftPlatform> {
    thread_local! {
        static PLATFORM: RefCell<Option<Rc<SoftPlatform>>> = const { RefCell::new(None) };
    }
    PLATFORM.with(|slot| {
        let mut slot = slot.borrow_mut();
        slot.get_or_insert_with(|| {
            let platform = Rc::new(SoftPlatform {
                pending: RefCell::new(Vec::new()),
            });
            // `set_platform` 要 `Box<dyn Platform>`，用一个共享句柄包一层
            slint::platform::set_platform(Box::new(SoftPlatformHandle(platform.clone())))
                .expect("设置软件渲染平台（每进程只允许一次）");
            platform
        })
        .clone()
    })
}

/// 把 thread-local 里持有的 `Rc<SoftPlatform>` 转成 `Platform` 实现交给 Slint。
struct SoftPlatformHandle(Rc<SoftPlatform>);

impl Platform for SoftPlatformHandle {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        self.0.create_window_adapter()
    }
}

fn render<T: ComponentHandle>(
    window: &Rc<MinimalSoftwareWindow>,
    ui: &T,
    width: u32,
    height: u32,
) -> Vec<[u8; 4]> {
    ui.window()
        .set_size(LogicalSize::new(width as f32, height as f32));
    slint::platform::update_timers_and_animations();
    window.request_redraw();
    // 软件渲染器的目标像素类型：`PremultipliedRgbaColor` 是它有实现且字段公开的那一个。
    let mut buffer = vec![PremultipliedRgbaColor::default(); (width * height) as usize];
    let redrawn = window.draw_if_needed(|renderer| {
        renderer.render(buffer.as_mut_slice(), width as usize);
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

fn pixel_at(frame: &[[u8; 4]], x: u32, y: u32, width: u32) -> [u8; 4] {
    frame[(y * width + x) as usize]
}

#[test]
fn radius_actually_rounds_corners() {
    // 必须先装好软件渲染平台：窗口组件 `::new()` 会去要一个窗口 adapter，
    // 没装平台时 Slint 会去初始化默认（winit）后端，而它要求事件循环在主线程。
    let platform = platform();
    let window = platform.new_window();
    let ui = RadiusProbe::new().expect("构造圆角探针窗口");

    // ① 直角（库默认）：三块面的左上角都被面自己覆盖（描边或硬阴影层），不是红底
    ui.set_probe_radius(0.0);
    let square = render(&window, &ui, WIDTH, HEIGHT);
    for (x, y, name) in PROBES {
        assert_ne!(
            pixel_at(&square, x, y, WIDTH)[..3],
            BACKDROP,
            "{name} 在 radius=0 时左上角应被面自身覆盖，实际露出了背景色（说明布局坐标与探针不一致）"
        );
    }

    // ② 圆角 12px / 小件 6px：同一个左上角像素必须被切掉，露出红底
    ui.set_probe_radius(12.0);
    let rounded = render(&window, &ui, WIDTH, HEIGHT);
    assert_ne!(
        square, rounded,
        "改 PixelTheme.radius 后渲染结果没变化：组件没读主题圆角"
    );
    for (x, y, name) in PROBES {
        assert_eq!(
            pixel_at(&rounded, x, y, WIDTH)[..3],
            BACKDROP,
            "{name} 在 radius>0 时左上角应被圆角切掉（露出背景），实际仍是面自身颜色"
        );
    }

    // ③ 改回直角应当复原 —— 排除「一次性、不可逆」的假通过
    ui.set_probe_radius(0.0);
    assert_eq!(
        render(&window, &ui, WIDTH, HEIGHT),
        square,
        "把 radius 设回 0px 后应回到直角渲染结果"
    );

    // ④ 圆角窗身：window-radius = 0 时全出血（四角是窗身面色），> 0 时四角露出窗口底色
    const BODY: u32 = 80;
    let body_window = platform.new_window();
    let body = WindowBodyProbe::new().expect("构造圆角窗身探针窗口");
    body.set_probe_window_radius(0.0);
    let full_bleed = render(&body_window, &body, BODY, BODY);
    assert_ne!(
        pixel_at(&full_bleed, 0, 0, BODY)[..3],
        BACKDROP,
        "window-radius=0 时窗身应全出血（左上角是面色），实际露了窗口底色"
    );
    body.set_probe_window_radius(12.0);
    let rounded_body = render(&body_window, &body, BODY, BODY);
    // (0,0) 只能证明窗身被内缩（pad）；(4,4) 还要求圆角真的把这一像素切掉：
    // pad=3 的方角窗身在 (4,4) 仍是面色，只有圆角（r=12）才会露出窗口底色。
    for (x, y) in [(0, 0), (4, 4)] {
        assert_eq!(
            pixel_at(&rounded_body, x, y, BODY)[..3],
            BACKDROP,
            "window-radius>0 时 ({x},{y}) 必须被圆角窗身切掉（露出窗口底色）"
        );
    }
    assert_ne!(
        pixel_at(&full_bleed, 4, 4, BODY)[..3],
        BACKDROP,
        "window-radius=0 时 (4,4) 应是窗身面色（全出血，没有透明边距）"
    );
    body.set_probe_window_radius(0.0);
    assert_eq!(
        render(&body_window, &body, BODY, BODY),
        full_bleed,
        "把 window-radius 设回 0px 后应回到全出血渲染结果"
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
    assert!(
        std::fs::read_to_string(ui_dir.join("pixel_theme.slint"))
            .expect("读取 pixel_theme.slint")
            .contains("in-out property <length> window-radius: 0px;"),
        "PixelTheme.window-radius 的默认值必须是 0px（直角窗口，且不要求宿主开透明窗口）"
    );
}

/// 描边宽度守卫：`ui/*.slint` 里不许再出现硬编码宽度（必须走 PixelTheme token），
/// 否则又会回到"卡片 2px / primary 3px / 输入框 0.5px"那种宿主改不动的混档。
///
/// 例外只有"字形框/轨道内部细节"这两类，它们的宽度与控件描边档位无关，逐条列明。
#[test]
fn every_border_width_is_theme_driven() {
    /// (文件, 该行内容, 理由)
    const EXEMPT: [(&str, &str, &str); 3] = [
        (
            "pixel_painter_widget.slint",
            "border-width: 1px;",
            "TitleButton 面：border-color 复用 face，纯字形内缩",
        ),
        (
            "pixel_p1.slint",
            "border-width: 1px;",
            "PixelRangeSlider 轨道内部细节",
        ),
        (
            "pixel_widgets.slint",
            "border-width: 1px;",
            "PixelSlider 轨道内部细节",
        ),
    ];

    let ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    let mut offenders = Vec::new();
    let mut checked = 0usize;

    for entry in std::fs::read_dir(&ui_dir).expect("读取 ui 目录") {
        let path = entry.expect("目录项").path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.extension().and_then(|e| e.to_str()) != Some("slint") || name == "pixel_theme.slint"
        {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("读取 .slint 源文件");
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if !trimmed.starts_with("border-width:") {
                continue;
            }
            checked += 1;
            let exempt = EXEMPT
                .iter()
                .any(|(f, l, _)| *f == name && trimmed.starts_with(l));
            if exempt || trimmed.contains("PixelTheme.") || trimmed.ends_with("0px;") {
                continue;
            }
            offenders.push(format!("{name}:{} {}", i + 1, trimmed));
        }
    }

    assert!(
        checked > 100,
        "覆盖面异常：只检查到 {checked} 处 border-width"
    );
    assert!(
        offenders.is_empty(),
        "以下描边宽度是硬编码的（宿主改不动，且容易和主题档位混档）：\n{}",
        offenders.join("\n")
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

/// 分隔线守卫（文本级，简单可控）：`ui/*.slint` 里用矩形画的细线（1~3px 的宽/高 + 描边色背景）
/// 必须读主题值，否则宿主把 `border-width` 调细时边框变细、分隔线仍是 2px —— owner 说的"容器类不一致"。
#[test]
fn divider_lines_follow_theme_border_width() {
    /// 状态高亮条（不是分隔线）：PixelButton 的 active 指示条
    const EXEMPT: [(&str, &str); 1] =
        [("pixel_painter_widget.slint", "// 底部高亮条（active 标记）")];

    let ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    let edge_colors = ["root.edge", "root.border", "root.line", "root.bubble-edge"];
    let literals = [
        "width: 1px",
        "width: 2px",
        "width: 3px",
        "height: 1px",
        "height: 2px",
        "height: 3px",
    ];
    let mut offenders = Vec::new();
    let mut checked = 0usize;

    for entry in std::fs::read_dir(&ui_dir).expect("读取 ui 目录") {
        let path = entry.expect("目录项").path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.extension().and_then(|e| e.to_str()) != Some("slint") || name == "pixel_theme.slint"
        {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("读取 .slint 源文件");
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            let has_edge_bg = edge_colors.iter().any(|c| t.contains(c));
            let has_literal_size = literals.iter().any(|l| t.contains(l));
            if !(has_edge_bg && has_literal_size) {
                continue;
            }
            // 同一行里已经用了主题值就不算（例如 `height: PixelTheme.border-width;`）
            if t.contains("PixelTheme.") {
                continue;
            }
            if EXEMPT
                .iter()
                .any(|(f, marker)| *f == name && t.contains(marker))
            {
                continue;
            }
            checked += 1;
            offenders.push(format!("{name}:{} {}", i + 1, t));
        }
    }

    assert!(
        offenders.is_empty(),
        "以下分隔线仍是硬编码宽度（应读 PixelTheme.border-width）：\n{}",
        offenders.join("\n")
    );
    let _ = checked;
}
