//! `PixelTitleBar.native-move`（Slint 1.18 `WindowMoveArea`）的契约测试。
//!
//! 两层把关：
//! 1. **结构守卫**（`window_move_area_wraps_title_bar_content`）：`WindowMoveArea` 必须是标题栏
//!    内容的**祖先**而不是同层兄弟。同层兄弟形态下，全宽 `TouchArea` 的 `GrabMouse` 会先吃掉按下并
//!    中止同层派发，`WindowMoveArea` 永远不会被武装 —— 表现为 native-move 配了却纹丝不动，
//!    而 `drag-start` 守卫同样不触发，所以"事件路由"类断言锁不住它（2026-09-18 独立审查实测）。
//! 2. **事件路由断言**（`native_move_contract`）：默认路径按下标题栏仍触发 `drag-start`；
//!    `native-move: true` 时不再触发；右侧关闭按钮在两种模式下都可用。
//!
//! 已知验证缺口：**"平台移动真的被发起"无法用公开 API 断言**——`WindowAdapter` 的公开 trait
//! 没有 `start_window_move()`（它在内核内部 trait 上，靠 `internal(InternalToken)` 暴露），
//! Slint testing backend 的 `window_move_request_count()` 又只在 `internal` feature 下公开，
//! 而该 feature 在 crates.io 包里编不过。该点由审查方用本地补丁版 testing backend 实测
//! （同级形态计数 0；容器形态拖动 40px 后计数 1），仓库内以结构守卫防回归。

use std::cell::Cell;
use std::rc::Rc;

use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle as _, LogicalPosition, LogicalSize, WindowSize};

fn press(ui: &slint_pixel::PixelPainterWindow, x: f32, y: f32) {
    ui.window()
        .dispatch_event_with_result(WindowEvent::PointerPressed {
            position: LogicalPosition::new(x, y),
            button: PointerEventButton::Left,
        })
        .expect("派发 PointerPressed");
}

fn moved(ui: &slint_pixel::PixelPainterWindow, x: f32, y: f32) {
    ui.window()
        .dispatch_event_with_result(WindowEvent::PointerMoved {
            position: LogicalPosition::new(x, y),
        })
        .expect("派发 PointerMoved");
}

fn release(ui: &slint_pixel::PixelPainterWindow, x: f32, y: f32) {
    ui.window()
        .dispatch_event_with_result(WindowEvent::PointerReleased {
            position: LogicalPosition::new(x, y),
            button: PointerEventButton::Left,
        })
        .expect("派发 PointerReleased");
}

/// 标题栏在 VerticalLayout 8px padding 之后、高 34px；x=80 落在标题文字区（不在右侧按钮上）。
const BAR_X: f32 = 80.0;
const BAR_Y: f32 = 20.0;
/// 关闭按钮在右侧 106px 区内，取靠近右缘处。
const CLOSE_X: f32 = 690.0;
const CLOSE_Y: f32 = 25.0;

#[test]
fn native_move_contract() {
    // 无头：用 Slint testing backend，避免测试去连真实显示后端。
    i_slint_backend_testing::init_no_event_loop();

    let ui = slint_pixel::PixelPainterWindow::new().expect("构造 PixelPainterWindow 实例");
    ui.window()
        .set_size(WindowSize::Logical(LogicalSize::new(720.0, 560.0)));

    let drag_starts = Rc::new(Cell::new(0));
    let closes = Rc::new(Cell::new(0));
    {
        let counter = drag_starts.clone();
        ui.on_drag_start(move || counter.set(counter.get() + 1));
    }
    {
        let counter = closes.clone();
        ui.on_close_window(move || counter.set(counter.get() + 1));
    }

    assert!(
        !ui.get_native_move(),
        "native-move 默认必须关闭，保证既有宿主行为不变"
    );

    // ① 默认路径：按下即 drag-start 回调（老契约，宿主自行 drag_window）
    press(&ui, BAR_X, BAR_Y);
    assert_eq!(drag_starts.get(), 1, "默认路径按下标题栏应触发 drag-start");
    release(&ui, BAR_X, BAR_Y);

    // ② native-move=true：drag-start 不再触发（移动改由 WindowMoveArea 负责）
    ui.set_native_move(true);
    assert!(ui.get_native_move());
    press(&ui, BAR_X, BAR_Y);
    moved(&ui, BAR_X + 40.0, BAR_Y);
    assert_eq!(
        drag_starts.get(),
        1,
        "native-move 打开后不应再触发 drag-start（不重复发起移动）"
    );
    release(&ui, BAR_X + 40.0, BAR_Y);

    // ③ 容器形态不能让右侧按钮失效：关闭按钮仍可点
    press(&ui, CLOSE_X, CLOSE_Y);
    release(&ui, CLOSE_X, CLOSE_Y);
    assert_eq!(closes.get(), 1, "native-move 打开后关闭按钮仍应可用");

    // ④ 关回默认：老路径恢复
    ui.set_native_move(false);
    press(&ui, BAR_X, BAR_Y);
    assert_eq!(
        drag_starts.get(),
        2,
        "关回 native-move 后应恢复 drag-start 回调"
    );
    release(&ui, BAR_X, BAR_Y);
}

/// 结构守卫：`PixelTitleBar` 里 `WindowMoveArea` 必须**包住** `TouchArea`（祖先而非同层兄弟）。
/// 兄弟形态会被 `TouchArea` 的 `GrabMouse` 抢先中止派发，窗口不会移动（见文件头说明）。
#[test]
fn window_move_area_wraps_title_bar_content() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/ui/pixel_painter_widget.slint"
    ))
    .expect("读取 pixel_painter_widget.slint");

    let lines: Vec<&str> = src.lines().collect();
    // 截取 PixelTitleBar 组件体：从 `export component PixelTitleBar` 到其后第一个顶格 `}`
    let start = lines
        .iter()
        .position(|l| l.starts_with("export component PixelTitleBar"))
        .expect("找到 PixelTitleBar 组件");
    let end = (start + 1..lines.len())
        .find(|&i| lines[i] == "}")
        .expect("找到 PixelTitleBar 组件结束");
    let body = &lines[start..end];

    let area = body
        .iter()
        .position(|l| l.trim_start().starts_with("WindowMoveArea {"))
        .expect("PixelTitleBar 里应声明 WindowMoveArea");
    // 用花括号配对找 WindowMoveArea 块的结尾
    let mut depth = 0i32;
    let mut area_end = None;
    for (i, l) in body.iter().enumerate().skip(area) {
        depth += l.matches('{').count() as i32 - l.matches('}').count() as i32;
        if depth <= 0 {
            area_end = Some(i);
            break;
        }
    }
    let area_end = area_end.expect("WindowMoveArea 块应闭合");

    let touch_area = body
        .iter()
        .position(|l| l.trim_start().starts_with("TouchArea {"))
        .expect("PixelTitleBar 里应声明 TouchArea");

    assert!(
        touch_area > area && touch_area < area_end,
        "WindowMoveArea 必须包住 TouchArea（祖先形态）：\n  WindowMoveArea 块 = 行 {area}..{area_end}\n  TouchArea 在第 {touch_area} 行\n\
         同层兄弟形态下 TouchArea 的 GrabMouse 会中止同层派发，native-move 会静默失效。"
    );
}
