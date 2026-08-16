#![deny(unsafe_code)]

use std::error::Error;

slint::include_modules!();

// 把生成的 MainWindow 类型适配到 slint-bitmap 的接线契约（固有方法优先于 trait 方法）
slint_bitmap::impl_painter_ui!(MainWindow);
slint_bitmap::impl_title_bar_ui!(MainWindow);

fn main() -> Result<(), Box<dyn Error>> {
    let ui = MainWindow::new()?;

    // 一键接线：画/擦、清空、导出 PNG + 标题栏窗口控制
    let _canvas = slint_bitmap::install_painter(&ui);
    slint_bitmap::install_title_bar_controls(&ui);

    ui.run()?;
    Ok(())
}
