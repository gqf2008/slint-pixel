fn main() {
    println!("cargo:rerun-if-changed=ui/lib.slint");
    println!("cargo:rerun-if-changed=ui/pixel_theme.slint");
    println!("cargo:rerun-if-changed=ui/pixel_presets.slint");
    println!("cargo:rerun-if-changed=ui/pixel_painter_widget.slint");
    println!("cargo:rerun-if-changed=ui/pixel_widgets.slint");
    println!("cargo:rerun-if-changed=ui/pixel_complex.slint");
    println!("cargo:rerun-if-changed=ui/pixel_extra.slint");
    println!("cargo:rerun-if-changed=ui/pixel_tailwind.slint");
    println!("cargo:rerun-if-changed=ui/pixel_painter_window.slint");

    // 与 src/lib.rs::library_paths() 保持一致：`@slint_pixel` -> 组件库汇总入口
    let config = || {
        let library_paths = std::collections::HashMap::from([(
            "slint_pixel".to_string(),
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/lib.slint"),
        )]);
        slint_build::CompilerConfiguration::new().with_library_paths(library_paths)
    };

    // 集成测试只能 include! 构建期生成的代码，圆角行为探针因此在这里编译（tests/theme_radius.rs 用）。
    // 必须排在 pixel_painter_window.slint 之前：SLINT_INCLUDE_GENERATED 指向最后一次编译的文件。
    slint_build::compile_with_config("tests/theme_radius.slint", config())
        .expect("编译圆角探针 tests/theme_radius.slint 失败");

    slint_build::compile_with_config("ui/pixel_painter_window.slint", config())
        .expect("编译 Slint UI 失败");
}
