#![deny(unsafe_code)]

use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

// build.rs 编译了两个入口（ui/main.slint 画板窗口、ui/gallery.slint 组件画廊），
// `slint::include_modules!()` 只会包含最后一个，这里手动包含两个生成文件。
include!(concat!(env!("OUT_DIR"), "/main.rs"));
include!(concat!(env!("OUT_DIR"), "/gallery.rs"));

// 把生成的窗口类型适配到组件库接线契约（固有方法优先于 trait 方法）
slint_pixel::impl_painter_ui!(MainWindow);
slint_pixel::impl_title_bar_ui!(MainWindow);
slint_pixel::impl_resize_ui!(MainWindow);
slint_pixel::impl_title_bar_ui!(GalleryWindow);
slint_pixel::impl_resize_ui!(GalleryWindow);

fn main() -> Result<(), Box<dyn Error>> {
    use slint::ComponentHandle;

    let gallery = GalleryWindow::new()?;
    slint_pixel::install_title_bar_controls(&gallery);
    slint_pixel::install_window_resize(&gallery);

    // 画廊里的“打开像素画板”：新开一个画板窗口并保持存活
    let painters: Rc<RefCell<Vec<MainWindow>>> = Rc::new(RefCell::new(Vec::new()));
    let painters_open = painters.clone();
    gallery.on_open_painter(move || {
        if let Ok(painter) = MainWindow::new() {
            slint_pixel::install_painter(&painter);
            slint_pixel::install_title_bar_controls(&painter);
            slint_pixel::install_window_resize(&painter);
            if painter.show().is_ok() {
                painters_open.borrow_mut().push(painter);
            }
        }
    });

    gallery.show()?;
    slint::run_event_loop()?;
    Ok(())
}
