fn main() {
    println!("cargo:rerun-if-changed=ui/lib.slint");
    println!("cargo:rerun-if-changed=ui/pixel_painter_widget.slint");
    println!("cargo:rerun-if-changed=ui/pixel_widgets.slint");
    println!("cargo:rerun-if-changed=ui/pixel_painter_window.slint");

    // 与 src/lib.rs::library_paths() 保持一致：`@slint_bitmap` -> 组件库汇总入口
    let library_paths = std::collections::HashMap::from([(
        "slint_bitmap".to_string(),
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/lib.slint"),
    )]);

    slint_build::compile_with_config(
        "ui/pixel_painter_window.slint",
        slint_build::CompilerConfiguration::new().with_library_paths(library_paths),
    )
    .expect("编译 Slint UI 失败");
}
