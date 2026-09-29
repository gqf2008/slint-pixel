//! 无头把组件画廊渲染成 PNG（不需要显示器、不需要 GPU）：用来生成 `docs/` 截图，
//! 或者在没有图形环境时肉眼检查外观。
//!
//! ```sh
//! cargo run -p slint-pixel-demo --example render_gallery -- /tmp/gallery-shots
//! cargo run -p slint-pixel-demo --example render_gallery -- /tmp/gallery-shots 2400   # 拉高窗口看完整内容
//! ```
//!
//! 每个预设输出一张：`gallery-classic.png` / `gallery-soft.png` / `gallery-dark.png`。

use std::path::PathBuf;
use std::rc::Rc;

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
};
use slint::platform::{Platform, PlatformError, WindowAdapter};
use slint::{ComponentHandle as _, LogicalSize};

include!(concat!(env!("OUT_DIR"), "/gallery.rs"));

const WIDTH: u32 = 760;
const DEFAULT_HEIGHT: u32 = 640;
const PRESETS: [&str; 3] = ["classic", "soft", "dark"];

struct SoftPlatform(Rc<MinimalSoftwareWindow>);

impl Platform for SoftPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.0.clone())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let out_dir = PathBuf::from(args.next().unwrap_or_else(|| ".".into()));
    let height: u32 = args
        .next()
        .map(|h| h.parse().expect("第二个参数是窗口高度（像素）"))
        .unwrap_or(DEFAULT_HEIGHT);
    std::fs::create_dir_all(&out_dir)?;

    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(SoftPlatform(window.clone())))?;

    let gallery = GalleryWindow::new()?;
    gallery
        .window()
        .set_size(LogicalSize::new(WIDTH as f32, height as f32));
    let _ = gallery.show();

    for preset in PRESETS {
        gallery.set_theme_preset(preset.into());
        slint::platform::update_timers_and_animations();
        window.request_redraw();

        let mut pixels = vec![PremultipliedRgbaColor::default(); (WIDTH * height) as usize];
        let redrawn = window.draw_if_needed(|renderer| {
            renderer.render(pixels.as_mut_slice(), WIDTH as usize);
        });
        assert!(redrawn, "软件渲染没有画出「{preset}」这一帧");

        let path = out_dir.join(format!("gallery-{preset}.png"));
        write_png(&path, height, &pixels)?;
        println!("{}", path.display());
    }
    Ok(())
}

fn write_png(
    path: &PathBuf,
    height: u32,
    pixels: &[PremultipliedRgbaColor],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rgba = Vec::with_capacity(pixels.len() * 4);
    for p in pixels {
        // 软件渲染器给的是预乘 alpha，PNG 要的是直通 alpha。窗口底色不透明，这里通常等于原值。
        let a = p.alpha as u32;
        let unpremultiply = |c: u8| {
            (c as u32 * 255 + a / 2)
                .checked_div(a)
                .unwrap_or(0)
                .min(255) as u8
        };
        rgba.extend_from_slice(&[
            unpremultiply(p.red),
            unpremultiply(p.green),
            unpremultiply(p.blue),
            p.alpha,
        ]);
    }
    let image = image::RgbaImage::from_raw(WIDTH, height, rgba).expect("像素数量与尺寸一致");
    image.save_with_format(path, image::ImageFormat::Png)?;
    Ok(())
}
