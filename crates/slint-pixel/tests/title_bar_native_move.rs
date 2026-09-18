//! `PixelTitleBar.native-move`（Slint 1.18 `WindowMoveArea`）的契约测试。
//!
//! 这里锁住的是**默认值**：默认必须是 `false`，老的宿主接线（`drag-start` → winit
//! `drag_window()`）行为才不会被这次改动悄悄改掉；打开后由 `WindowMoveArea` 接管，
//! 且组件不再触发 `drag-start`（避免同一次拖动发起两遍）。
//!
//! 这里用 Slint 的 testing backend 做无头合成鼠标事件，锁住**事件路由契约**：
//! 默认（`native-move: false`）按下标题栏仍然触发 `drag-start`（老宿主接线不变）；
//! 打开后不再触发（移动由 `WindowMoveArea` 交给平台，避免同一次拖动发起两遍）。
//!
//! 已知验证缺口：窗口**真的跟着鼠标移动**这一步无法在无头环境断言——Slint testing backend 的
//! `TestingWindow::window_move_request_count()` 只在 `internal` feature 下公开，公开 API 拿不到
//! "平台移动被发起"的证据。真机检查步骤见 README「像素标题栏」一节。

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

fn release(ui: &slint_pixel::PixelPainterWindow, x: f32, y: f32) {
    ui.window()
        .dispatch_event_with_result(WindowEvent::PointerReleased {
            position: LogicalPosition::new(x, y),
            button: PointerEventButton::Left,
        })
        .expect("派发 PointerReleased");
}

#[test]
fn native_move_toggles_drag_start_contract() {
    // 无头：用 Slint 的 testing backend，避免测试去连真实显示后端。
    i_slint_backend_testing::init_no_event_loop();

    let ui = slint_pixel::PixelPainterWindow::new().expect("构造 PixelPainterWindow 实例");
    ui.window()
        .set_size(WindowSize::Logical(LogicalSize::new(720.0, 560.0)));

    let drag_starts = Rc::new(Cell::new(0));
    {
        let counter = drag_starts.clone();
        ui.on_drag_start(move || counter.set(counter.get() + 1));
    }

    // 标题栏位于 VerticalLayout 8px padding 之后、高 34px；x=80 落在标题文字区（不在右侧按钮上）。
    const X: f32 = 80.0;
    const Y: f32 = 20.0;

    assert!(
        !ui.get_native_move(),
        "native-move 默认必须关闭，保证既有宿主行为不变"
    );

    press(&ui, X, Y);
    assert_eq!(
        drag_starts.get(),
        1,
        "默认路径：按下标题栏应触发 drag-start（老契约）"
    );
    release(&ui, X, Y);

    ui.set_native_move(true);
    assert!(ui.get_native_move(), "native-move 打开后应能读回 true");
    press(&ui, X, Y);
    assert_eq!(
        drag_starts.get(),
        1,
        "native-move 打开后不应再触发 drag-start（避免重复发起移动）"
    );
    release(&ui, X, Y);

    ui.set_native_move(false);
    press(&ui, X, Y);
    assert_eq!(
        drag_starts.get(),
        2,
        "关回 native-move 后应恢复 drag-start 回调"
    );
    release(&ui, X, Y);
}
