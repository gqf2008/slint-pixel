#[test]
fn ui_components_compile_smoke() {
    let out = std::env::temp_dir().join(format!("slint-pixel-smoke-{}", std::process::id()));
    std::fs::create_dir_all(&out).expect("create OUT_DIR");
    std::env::set_var("OUT_DIR", &out);
    let library_paths = slint_pixel::library_paths();
    let config = slint_build::CompilerConfiguration::new().with_library_paths(library_paths);
    slint_build::compile_with_config(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/ui_smoke.slint"),
        config,
    )
    .expect("UI 组件 compile smoke 失败");
}
