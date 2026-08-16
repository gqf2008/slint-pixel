fn main() {
    println!("cargo:rerun-if-changed=ui/pixel_painter.slint");
    slint_build::compile("ui/pixel_painter.slint").expect("编译 Slint UI 失败");
}
