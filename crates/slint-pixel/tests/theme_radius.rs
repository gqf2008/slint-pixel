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
    // 统一 2px 批次后滑轨已改走 PixelTheme.border-width，全库仅剩 TitleButton 一处 1px
    // （border-color 复用 face 的字形内缩，不是框线）；其余任何文件新出现 1px 都会在此变红。
    const EXEMPT: [(&str, &str, &str); 1] = [(
        "pixel_painter_widget.slint",
        "border-width: 1px;",
        "TitleButton 面：border-color 复用 face，纯字形内缩",
    )];

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

/// 3px 线条守卫：任何**带背景色的可见线条**都不允许硬编码 3px（横线看 height、竖线看 width）。
/// 约定是全库线条一律跟随 PixelTheme.border-width（当前 2px）——tabs/navbar 激活条、
/// 选中指示条这类"强调线"历史上两次滑回 3px，与 2px 边框同框即混档；3px 在本体系里
/// 没有合法用途（滑轨填充/字形块不是线条，走不到这个断言）。
///
/// 检测要覆盖该写法的全部形状（sp-pkg-r 评审 P1 实证过漏报）：
/// 两行式、`Rectangle { height: 3px; background: …; }` 单行元素、
/// 多行元素内 `height: 3px; background: …;` 同行、`height: 3px` 无分号（右花括号前省略）。
/// 探针文件 zz_guard_probe.slint 做过四形状阳性对照（修复前只抓两行式，修复后四种全红）。
#[test]
fn no_hardcoded_3px_painted_lines() {
    let ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    let mut offenders = Vec::new();

    for entry in std::fs::read_dir(&ui_dir).expect("读取 ui 目录") {
        let path = entry.expect("目录项").path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.extension().and_then(|e| e.to_str()) != Some("slint") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("读取 .slint 源文件");
        let lines: Vec<&str> = source.lines().collect();
        let blocks = enclosing_block(&lines);
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            // 按 `;`/`{`/`}` 切段后精确匹配尺寸段：兼容无分号、同行多属性、单行元素；
            // `preferred-height:` 这类带前缀的属性名不会被误判
            let three_px = t
                .split([';', '{', '}'])
                .any(|s| matches!(s.trim(), "height: 3px" | "width: 3px"));
            if !three_px {
                continue;
            }
            // painted：本行带 background（单行元素/同行式）→ 命中；否则回退经典两行式
            // （background 单独一行、与尺寸行同元素块）
            let painted = t.contains("background:")
                || lines
                    .iter()
                    .enumerate()
                    .any(|(j, l)| blocks[j] == blocks[i] && l.trim().starts_with("background:"));
            if painted {
                offenders.push(format!("{name}:{} {}", i + 1, t));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "以下可见线条硬编码 3px（应跟随 PixelTheme.border-width，与 2px 边框同框才不混档）：\n{}",
        offenders.join("\n")
    );
}

/// 出血阴影守卫：`字面量 x/y 平移 + 撑满 parent（或 100%）+ 不透明背景` 的矩形就是旧的
/// "像素键阴影"写法——border-width 升到 2px 后，阴影层让右/下缘比左/上缘粗一倍
/// （owner 原话"线框粗细不一致"的根因之一）。统一 2px 体系里该写法废除：
/// 要么整面同色走 border（平顶直框），要么背景 transparent 只做布局占位。
///
/// 检测形状（sp-pkg-r 评审风格，逐形状阳性对照过）：
/// 同行单行元素 / 多行连写 / x·y 与 w·h 全分行（旧 PixelButton 像素键）/ 100% 撑满。
/// 字面量只认 `x: 2px` 这种纯数字段——条件偏移（`x: ta.pressed ? 2px : 0px`）不可文本
/// 求值，不在本守卫范围（按压下沉是瞬态反馈，另有视觉审计兜底）。
/// 阴性对照（当前代码库真实存在、不得误报）：transparent 占位、定长内衬（色板/画布框）、
/// `parent.width - Npx` 内缩（ScrollView 内容区/滑轨填充）、条件偏移。
#[test]
fn no_bleeding_shadow_rects() {
    /// 从 `;`/`{`/`}` 切段里取 `轴: 数字px` 的字面量（条件式解析失败 → None）
    fn lit_px(segs: &[&str], axis: &str) -> Option<i64> {
        segs.iter()
            .filter_map(|s| {
                let rest = s.trim().strip_prefix(axis)?.strip_prefix(": ")?.trim();
                rest.strip_suffix("px")?.trim().parse::<i64>().ok()
            })
            .max()
    }
    fn is_full_w(s: &str) -> bool {
        matches!(s.trim(), "width: parent.width" | "width: 100%")
    }
    fn is_full_h(s: &str) -> bool {
        matches!(s.trim(), "height: parent.height" | "height: 100%")
    }

    let ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    let mut offenders: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut anchors = 0usize;

    for entry in std::fs::read_dir(&ui_dir).expect("读取 ui 目录") {
        let path = entry.expect("目录项").path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.extension().and_then(|e| e.to_str()) != Some("slint") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("读取 .slint 源文件");
        let lines: Vec<&str> = source.lines().collect();
        let blocks = enclosing_block(&lines);
        for (i, line) in lines.iter().enumerate() {
            // 窗口 = 本行 + 同元素块内后 5 行（属性惯用紧随其后的连写/分行两种形态）；
            // 锚定行本身带 `{` 时元素块从本行起算，覆盖"开标签行内写 x/y"的形态
            let scope = if line.contains('{') { i } else { blocks[i] };
            let mut segs: Vec<&str> = line.split([';', '{', '}']).collect();
            for j in (i + 1)..=(i + 5).min(lines.len().saturating_sub(1)) {
                // 含 `{` 的行是子元素开标签：其行内属性属于子元素本身，
                // 由子元素行自行锚定（单行元素 self-anchor，多行元素从 `{` 行起块），跳过以免并集串味
                if blocks[j] == scope && !lines[j].contains('{') {
                    segs.extend(lines[j].split([';', '{', '}']));
                }
            }
            let x_off = lit_px(&segs, "x").is_some_and(|v| v > 0);
            let y_off = lit_px(&segs, "y").is_some_and(|v| v > 0);
            if !(x_off && y_off) {
                continue;
            }
            anchors += 1;
            let full = segs.iter().any(|s| is_full_w(s)) && segs.iter().any(|s| is_full_h(s));
            let opaque_bg = segs.iter().any(|s| {
                s.trim()
                    .strip_prefix("background:")
                    .is_some_and(|b| b.trim() != "transparent")
            });
            if full && opaque_bg && seen.insert((name.clone(), scope)) {
                offenders.push(format!("{name}:{} {}", i + 1, line.trim()));
            }
        }
    }

    assert!(
        anchors > 10,
        "覆盖面异常：只锚定到 {anchors} 处字面量偏移（守卫可能没在扫）"
    );
    assert!(
        offenders.is_empty(),
        "以下矩形是出血阴影（x/y 平移 + 撑满 parent + 不透明背景），右/下缘会比左/上缘粗：\n{}",
        offenders.join("\n")
    );
}

/// 显式边框色守卫：任何元素块里出现非 0 的 `border-width` 就必须有 `border-color`。
/// Slint 的 Rectangle 默认 border-color 是 **transparent**——只写 border-width 不写的
/// 边框永远不渲染。2026-09-30 统一 2px 批次的回归实证：批量把阴影层改 transparent 时，
/// 50 个面层带着"无色的 border-width"，框线整体消失（owner 报"组件边框都没了"），
/// 而宽度守卫/线粗守卫/渲染暗带审计全都抓不到"缺失"（审计只能抓多出来的，抓不住没了的）。
/// 0px（tailwind 的 `border-width: 0px` 占位）与 pixel_theme.slint 的 token 定义豁免。
/// 阳性对照：zz_border_color_probe.slint（同行单行/多行连写两种必抓 + 阴性两例不误报）。
#[test]
fn every_border_has_explicit_color() {
    let ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    let mut offenders = Vec::new();

    for entry in std::fs::read_dir(&ui_dir).expect("读取 ui 目录") {
        let path = entry.expect("目录项").path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.extension().and_then(|e| e.to_str()) != Some("slint") || name == "pixel_theme.slint"
        {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("读取 .slint 源文件");
        let lines: Vec<&str> = source.lines().collect();
        // 块起点 + 每个起点的块结束行
        let mut stack: Vec<usize> = Vec::new();
        let mut start_of = vec![usize::MAX; lines.len()];
        let mut end_of: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        for (i, line) in lines.iter().enumerate() {
            start_of[i] = *stack.last().unwrap_or(&usize::MAX);
            for ch in line.chars() {
                match ch {
                    '{' => stack.push(i),
                    '}' => {
                        if let Some(s) = stack.pop() {
                            end_of.insert(s, i);
                        }
                    }
                    _ => {}
                }
            }
        }
        for (i, line) in lines.iter().enumerate() {
            // 按 `;`/`{`/`}` 切段找 border-width 段；0px 是纯占位（视觉上无边），豁免
            let segs: Vec<&str> = line.split([';', '{', '}']).collect();
            let has_bw = segs.iter().any(|s| {
                let t = s.trim();
                t.starts_with("border-width:") && !t.ends_with("0px")
            });
            if !has_bw {
                continue;
            }
            // 含 `{` 的行（单行元素/开标签行）块从本行起算，否则取最内层未闭合块
            let st = if line.contains('{') { i } else { start_of[i] };
            let e = if st == usize::MAX {
                i
            } else {
                *end_of.get(&st).unwrap_or(&i)
            };
            let blk = lines[st.min(i)..=e.max(i)].join("\n");
            if !blk.contains("border-color") {
                offenders.push(format!("{name}:{} {}", i + 1, line.trim()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "以下元素写了 border-width 却没有 border-color（Slint 默认边框色 = transparent，框不会渲染）：\n{}",
        offenders.join("\n")
    );
}

/// 主题纯净度守卫：`ui/*.slint`（主题/预设文件自身除外）不许出现硬编码色值 ——
/// 皮肤色必须走 PixelTheme token，宿主改一处全库生效（库输出统一风格模版）。
/// 豁免清单只有一类：内容数据（二维码/条码的符号色、取色器默认数据、画板 PICO-8 品牌色），
/// 按「文件名 + 整行 trimmed 文本」精确登记；改动其中任何一行都会让守卫变红，强制重新评审。
#[test]
fn no_hardcoded_colors_outside_theme_files() {
    /// 从 `#` 起连续取十六进制字符；长度 ∈ {3,4,6,8} 且后界非十六进制字符即视为色值。
    fn hex_color_at(line: &str) -> Option<String> {
        let b = line.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'#' {
                let mut j = i + 1;
                while j < b.len() && b[j].is_ascii_hexdigit() {
                    j += 1;
                }
                if matches!(j - i - 1, 3 | 4 | 6 | 8) {
                    return Some(line[i..j].to_string());
                }
            }
            i += 1;
        }
        None
    }

    // (文件, 整行 trimmed) —— 内容数据豁免，逐行登记理由：
    // QR/条码符号色（p5/p8 各一对 dark/light）——功能符号，反色会不可扫；
    // 取色器默认数据（more/p7 的 palette/colors/value/hex 字符串）——被拾取内容不是皮肤；
    // 画板 PICO-8 品牌色与笔刷状态（painter_widget/window）——工具内容色，同族豁免。
    const EXEMPT: &[(&str, &str)] = &[
        ("pixel_more.slint", "in property <[color]> colors: [#000000, #ffffff, #ff004d, #ffa500, #ffe14d, #00c853, #00bcd4, #2979ff];"),
        ("pixel_more.slint", "in property <[color]> palette: [#000000, #ffffff, #ff004d, #ffa500, #ffe14d, #00c853, #00bcd4, #2979ff];"),
        ("pixel_more.slint", "in-out property <color> value: #ff004d;"),
        ("pixel_p5.slint", "in property <color> dark: #000000;"),
        ("pixel_p5.slint", "in property <color> light: #ffffff;"),
        ("pixel_p7.slint", "in-out property <color> value: #ff004d;"),
        ("pixel_p7.slint", "in-out property <string> hex: \"#ff004d\";"),
        ("pixel_p7.slint", "palette: [#ff004d, #ffa500, #ffe14d, #00c853, #00bcd4, #2979ff];"),
        ("pixel_p8.slint", "in property <color> dark: #000000;"),
        ("pixel_p8.slint", "in property <color> light: #ffffff;"),
        ("pixel_painter_widget.slint", "#000000, #1d2b53, #7e2553, #008751, #ab5236, #5f574f, #c2c3c7, #fff1e8,"),
        ("pixel_painter_widget.slint", "#ff004d, #ffa300, #ffec27, #00e436, #29adff, #83769c, #ff77a8, #ffccaa"),
        ("pixel_painter_widget.slint", "in-out property <color> brush-color: #ff004d;"),
        ("pixel_painter_widget.slint", "face-active: #ff2f77;"),
        ("pixel_painter_window.slint", "in-out property <color> brush-color: #ff004d;"),
    ];

    let ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    let mut offenders = Vec::new();
    for entry in std::fs::read_dir(&ui_dir).expect("读取 ui 目录") {
        let path = entry.expect("目录项").path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if path.extension().and_then(|e| e.to_str()) != Some("slint")
            || matches!(name, "pixel_theme.slint" | "pixel_presets.slint")
        {
            continue;
        }
        for (i, line) in std::fs::read_to_string(&path)
            .expect("读取 .slint 源文件")
            .lines()
            .enumerate()
        {
            if let Some(hex) = hex_color_at(line) {
                if !EXEMPT.contains(&(name, line.trim())) {
                    offenders.push(format!("{}:{}: {} （{}）", name, i + 1, line.trim(), hex));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "ui/*.slint 出现 {} 处硬编码色值（应走 PixelTheme token）：\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

/// 预设覆盖守卫：pixel_presets.slint 里 names 清单登记的每一套预设函数，都必须写全
/// PixelTheme 的全部 token（常量 on-ink/scrim、派生 radius-sm、宿主窗口件 window-radius
/// 除外——预设故意不覆写它们）。漏写会让 token 回落到 scheme 条件默认值（隐蔽翻车），
/// 拼错名字则静默赋给一个不存在的属性，两类都是文本扫描能钉死的 bug。
#[test]
fn every_preset_writes_every_token() {
    let ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    let theme_src =
        std::fs::read_to_string(ui_dir.join("pixel_theme.slint")).expect("读取 pixel_theme.slint");
    let presets_src = std::fs::read_to_string(ui_dir.join("pixel_presets.slint"))
        .expect("读取 pixel_presets.slint");

    // 主题声明的全部 token 名
    let mut theme_tokens: Vec<String> = Vec::new();
    for line in theme_src.lines() {
        let t = line.trim_start();
        // scheme 是 in property，其余是 in-out property，两种都要
        let rest = t
            .strip_prefix("in-out property <")
            .or_else(|| t.strip_prefix("in property <"));
        if let Some(rest) = rest {
            if let Some(gt) = rest.find('>') {
                let after = rest[gt + 1..].trim_start();
                let name: String = after
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '-')
                    .collect();
                if !name.is_empty() {
                    theme_tokens.push(name);
                }
            }
        }
    }
    assert!(
        !theme_tokens.is_empty(),
        "没从 pixel_theme.slint 解析到任何 token"
    );

    // 预设故意不覆写的：两枚常量 token + 派生 radius-sm + 宿主窗口件 window-radius
    const NOT_WRITTEN: &[&str] = &["on-ink", "scrim", "radius-sm", "window-radius"];
    let required: Vec<&str> = theme_tokens
        .iter()
        .map(|s| s.as_str())
        .filter(|t| !NOT_WRITTEN.contains(t))
        .collect();

    // 预设名单（names 与 labels 平行、顺序一致）
    let names_line = presets_src
        .lines()
        .find(|l| l.contains("in property <[string]> names:"))
        .expect("pixel_presets.slint 缺少 names 清单");
    let names: Vec<String> = names_line
        .split("names:")
        .nth(1)
        .and_then(|rest| rest.split('[').nth(1))
        .map(|inner| {
            inner
                .split(']')
                .next()
                .unwrap()
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .expect("names 清单解析失败");
    assert!(!names.is_empty(), "names 清单为空");

    let mut offenders = Vec::new();
    for name in &names {
        // 取 public function <name>( 的完整块（花括号配平）
        let mut lines = presets_src.lines();
        let mut block = String::new();
        let mut in_block = false;
        let mut depth = 0i32;
        for line in &mut lines {
            if !in_block {
                if line.starts_with("    public function ") && line.contains(&format!("{name}(")) {
                    in_block = true;
                    depth = line.matches('{').count() as i32 - line.matches('}').count() as i32;
                    block.push_str(line);
                    block.push('\n');
                }
            } else {
                depth += line.matches('{').count() as i32 - line.matches('}').count() as i32;
                block.push_str(line);
                block.push('\n');
                if depth <= 0 {
                    break;
                }
            }
        }
        if !in_block {
            offenders.push(format!("{name}: 找不到预设函数"));
            continue;
        }

        // 收集块内 PixelTheme.X = 赋值
        let mut assigned: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for line in block.lines() {
            let t = line.trim_start();
            if let Some(rest) = t.strip_prefix("PixelTheme.") {
                let tok: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '-')
                    .collect();
                if rest[tok.len()..].trim_start().starts_with('=') {
                    assigned.insert(tok);
                }
            }
        }
        for req in &required {
            if !assigned.contains(*req) {
                offenders.push(format!("{name}: 漏写 token {req}"));
            }
        }
        for a in &assigned {
            if !theme_tokens.contains(a) {
                offenders.push(format!("{name}: 赋给不存在的 token {a}（拼写错误）"));
            } else if NOT_WRITTEN.contains(&a.as_str()) {
                offenders.push(format!("{name}: 不应覆写 {a}（常量/派生/窗口件）"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "预设 token 覆盖问题 {} 处：\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}
