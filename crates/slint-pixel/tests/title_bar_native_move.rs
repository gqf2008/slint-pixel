//! `PixelTitleBar.native-move`（Slint 1.18 `WindowMoveArea`）的契约测试。
//!
//! 两层把关：
//! 1. **结构守卫**（`window_move_area_wraps_title_bar_content`）：`WindowMoveArea` 必须是标题栏
//!    内容的**祖先**而不是同层兄弟。同层兄弟形态下，全宽 `TouchArea` 的 `GrabMouse` 会先吃掉按下并
//!    中止同层派发，`WindowMoveArea` 永远不会被武装 —— 表现为 native-move 配了却纹丝不动，
//!    而 `drag-start` 守卫同样不触发，所以"事件路由"类断言锁不住它（2026-09-18 独立审查实测）。
//! 2. **事件路由断言**（`native_move_contract`）：默认路径按下标题栏仍触发 `drag-start`；
//!    `native-move: true` 时不再触发；右侧关闭按钮在两种模式下都可用。
//! 3. **行为守卫**（`native_move_arms_platform_move_intercept`）：`native-move: true` 时"两次越阈值
//!    拖拽"不该被算作双击（越阈值会被平台移动层 `Intercept`，给已 grab 的子 `TouchArea` 发 `Exit`），
//!    而"原地双击"仍应最大化。结构守卫只能盯住源码形状，这一条盯住**行为**：退回同层兄弟形态时它会报红。
//!
//! 验证强度说明：**"平台移动真的被发起"本身没有公开 API 可直接断言**——`WindowAdapter` 的公开
//! trait 没有 `start_window_move()`（它在内核内部 trait 上，靠 `internal(InternalToken)` 暴露），
//! 而 Slint testing backend 的 `window_move_request_count()` 需要 `internal` feature，该 feature
//! 在 crates.io 包里编不过。因此仓库内用上面第 3 条行为守卫做**间接**证据（拖拽不再算双击），
//! 直接计数由审查方用本地补丁版 testing backend 实测（同层兄弟形态恒为 0；容器形态拖动后为 1）。

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

/// 行为守卫：`native-move: true` 时平台移动必须**真的被武装**。
///
/// 判据是公开 API 可观测的：越过阈值时 `WindowMoveArea` 返回 `Intercept`，会给已持有 grab 的子
/// `TouchArea` 发 `Exit`，于是"按下→拖过阈值→松开"不再被计入点击序列。同一段手势在两种模式下结果
/// 不同（2026-09-18 实测）：`native-move: false` 时子 `TouchArea` 一直持有 grab，两次拖拽累计成一次
/// 双击 → `toggle-maximize` 触发 1 次；`native-move: true` 时 0 次。退回同层兄弟形态（或删掉
/// `WindowMoveArea`）会变回 1 次，本测试即报红——这正是结构守卫盯不住的回归。
#[test]
fn native_move_arms_platform_move_intercept() {
    i_slint_backend_testing::init_no_event_loop();

    let ui = slint_pixel::PixelPainterWindow::new().expect("构造 PixelPainterWindow 实例");
    ui.window()
        .set_size(WindowSize::Logical(LogicalSize::new(720.0, 560.0)));

    let toggles = Rc::new(Cell::new(0usize));
    {
        let counter = toggles.clone();
        ui.on_toggle_maximize(move || counter.set(counter.get() + 1));
    }

    ui.set_native_move(true);

    // 两次"按下 → 拖过阈值（8px）→ 松开"
    for _ in 0..2 {
        press(&ui, BAR_X, BAR_Y);
        moved(&ui, BAR_X + 60.0, BAR_Y);
        moved(&ui, BAR_X + 220.0, BAR_Y);
        release(&ui, BAR_X + 220.0, BAR_Y);
    }
    assert_eq!(
        toggles.get(),
        0,
        "native-move 打开时，越阈值的拖拽应被平台移动层拦下（子 TouchArea 收到 Exit），不该算作双击；\
         这里变成 1 说明 WindowMoveArea 没有被武装（例如退回同层兄弟形态）"
    );

    // 对照：原地双击仍应最大化（双击语义没有被这次改动改坏）
    for _ in 0..2 {
        press(&ui, BAR_X, BAR_Y);
        release(&ui, BAR_X, BAR_Y);
    }
    assert_eq!(
        toggles.get(),
        1,
        "native-move 打开时，原地双击标题栏仍应触发 toggle-maximize"
    );
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
