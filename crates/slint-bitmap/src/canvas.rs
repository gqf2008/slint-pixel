//! 像素画布：保存 16×16 网格级 RGBA 数据，负责放大渲染与 PNG 导出。
//!
//! 与 UI 的约定：
//! - `GRID` 为网格边长（格数）；
//! - `CELL_PX` 为每个格子在屏幕上占的逻辑像素边长；
//! - 显示缓冲为 `GRID * CELL_PX` 见方的 RGBA 位图，交给 Slint 以
//!   `image-rendering: pixelated` 放大呈现。

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

/// 画布网格边长（格数）。
pub const GRID: usize = 16;
/// 每个像素格在屏幕上占的逻辑像素边长。
pub const CELL_PX: usize = 24;

/// 空像素（透明）。
const EMPTY: [u8; 4] = [0, 0, 0, 0];

/// 画布：网格级 RGBA 数据。
pub struct Canvas {
    grid: usize,
    cell_px: usize,
    cells: Vec<[u8; 4]>,
}

impl Canvas {
    pub fn new(grid: usize, cell_px: usize) -> Self {
        Self {
            grid,
            cell_px,
            cells: vec![EMPTY; grid * grid],
        }
    }

    /// 用 `size×size`（格数）的方形笔刷在 `(col, row)` 落笔。
    /// `color` 为 `None` 表示擦除（变透明），越界自动裁剪。
    pub fn paint_brush(&mut self, col: usize, row: usize, size: usize, color: Option<[u8; 4]>) {
        let size = size.max(1) as i64;
        let half = size / 2;
        let grid = self.grid as i64;
        for dy in 0..size {
            for dx in 0..size {
                let c = col as i64 - half + dx;
                let r = row as i64 - half + dy;
                if (0..grid).contains(&c) && (0..grid).contains(&r) {
                    let idx = r as usize * self.grid + c as usize;
                    self.cells[idx] = color.unwrap_or(EMPTY);
                }
            }
        }
    }

    /// 清空画布。
    pub fn clear(&mut self) {
        self.cells.fill(EMPTY);
    }

    /// 读取网格坐标对应的 RGBA 像素（仅测试使用）。
    #[cfg(test)]
    pub fn cell(&self, col: usize, row: usize) -> [u8; 4] {
        self.cells[row * self.grid + col]
    }

    /// 渲染整块画布为 Slint 图片：
    /// 空格子画棋盘格，已画格子画立体像素块，再叠加像素网格线。
    pub fn render_display(&self) -> Image {
        let size = (self.grid * self.cell_px) as u32;
        let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(size, size);
        {
            let bytes = buf.make_mut_bytes();
            for r in 0..self.grid {
                for c in 0..self.grid {
                    self.fill_cell(bytes, size, c, r);
                }
            }
            self.draw_grid(bytes, size);
        }
        Image::from_rgba8(buf)
    }

    /// 把画布导出为网格分辨率的 PNG。
    pub fn export_png(&self, path: &std::path::Path) -> Result<(), String> {
        let mut data = Vec::with_capacity(self.cells.len() * 4);
        for px in &self.cells {
            data.extend_from_slice(px);
        }
        let img = image::RgbaImage::from_raw(self.grid as u32, self.grid as u32, data)
            .ok_or_else(|| "无法创建 PNG 缓冲".to_string())?;
        img.save(path).map_err(|e| format!("保存 PNG 失败: {e}"))
    }

    /// 填充单个格子的显示区域。
    fn fill_cell(&self, bytes: &mut [u8], img_size: u32, col: usize, row: usize) {
        let cell = self.cell_px as i32;
        let x0 = (col * self.cell_px) as i32;
        let y0 = (row * self.cell_px) as i32;
        let checker = (col + row).is_multiple_of(2);
        let (light, dark) = if checker {
            ([208, 208, 220, 255], [186, 186, 200, 255])
        } else {
            ([186, 186, 200, 255], [208, 208, 220, 255])
        };
        let px = self.cells[row * self.grid + col];

        for y in 0..cell {
            for x in 0..cell {
                let color = if px[3] == 0 {
                    if checker {
                        light
                    } else {
                        dark
                    }
                } else {
                    let mut color = px;
                    // 左上高光、右下阴影，做出立体像素块
                    if x < 2 || y < 2 {
                        color = shade(color, 1.18);
                    }
                    if x >= cell - 3 || y >= cell - 3 {
                        color = shade(color, 0.68);
                    }
                    color
                };
                let gx = (x0 + x) as u32;
                let gy = (y0 + y) as u32;
                let i = ((gy * img_size + gx) * 4) as usize;
                bytes[i..i + 4].copy_from_slice(&color);
            }
        }
    }

    /// 叠加 1px 像素网格线。
    fn draw_grid(&self, bytes: &mut [u8], img_size: u32) {
        let line = [72, 66, 96, 255];
        let mut put = |x: u32, y: u32| {
            let i = ((y * img_size + x) * 4) as usize;
            bytes[i..i + 4].copy_from_slice(&line);
        };
        let cell = self.cell_px as i32;
        for k in 1..=self.grid as i32 {
            let x = (k * cell - 1) as u32;
            for y in 0..img_size {
                put(x, y);
            }
            let yy = (k * cell - 1) as u32;
            for x in 0..img_size {
                put(x, yy);
            }
        }
    }
}

/// 按系数缩放 RGB（保持 alpha），用于高光/阴影。
fn shade(color: [u8; 4], factor: f32) -> [u8; 4] {
    let f = |v: u8| (v as f32 * factor).round().clamp(0.0, 255.0) as u8;
    [f(color[0]), f(color[1]), f(color[2]), color[3]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_sets_cell() {
        let mut c = Canvas::new(16, 24);
        c.paint_brush(3, 4, 1, Some([255, 0, 77, 255]));
        assert_eq!(c.cell(3, 4), [255, 0, 77, 255]);
        assert_eq!(c.cell(3, 3), EMPTY);
    }

    #[test]
    fn brush_clamps_at_edges() {
        let mut c = Canvas::new(16, 24);
        c.paint_brush(0, 0, 3, Some([0, 0, 0, 255]));
        // 3×3 居中于 (0,0)，实际覆盖 (0..=1, 0..=1)
        assert_eq!(c.cell(0, 0), [0, 0, 0, 255]);
        assert_eq!(c.cell(1, 1), [0, 0, 0, 255]);
        assert_eq!(c.cell(2, 2), EMPTY);
    }

    #[test]
    fn erase_clears_cell() {
        let mut c = Canvas::new(16, 24);
        c.paint_brush(8, 8, 1, Some([1, 2, 3, 255]));
        c.paint_brush(8, 8, 1, None);
        assert_eq!(c.cell(8, 8), EMPTY);
    }

    #[test]
    fn clear_resets_all() {
        let mut c = Canvas::new(16, 24);
        c.paint_brush(5, 5, 3, Some([9, 9, 9, 255]));
        c.clear();
        assert!(c.cells.iter().all(|&p| p == EMPTY));
    }

    #[test]
    fn export_png_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("slint_bitmap_test_export.png");
        let mut c = Canvas::new(16, 24);
        c.paint_brush(0, 0, 1, Some([255, 0, 77, 255]));
        c.export_png(&path).unwrap();
        let img = image::open(&path).unwrap().to_rgba8();
        assert_eq!(img.width(), 16);
        assert_eq!(img.height(), 16);
        assert_eq!(img.get_pixel(0, 0).0, [255, 0, 77, 255]);
        // 空格子导出为透明
        assert_eq!(img.get_pixel(15, 15).0, [0, 0, 0, 0]);
        std::fs::remove_file(&path).ok();
    }
}
