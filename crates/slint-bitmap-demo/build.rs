fn main() {
    println!("cargo:rerun-if-changed=ui/main.slint");
    println!("cargo:rerun-if-changed=ui/gallery.slint");
    // 下游消费者标准写法：注册 @slint_bitmap 库路径（本库所有 .slint 依赖也会被监听）
    let library_paths = slint_bitmap::library_paths();
    for path in library_paths.values() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let config = slint_build::CompilerConfiguration::new().with_library_paths(library_paths);
    slint_build::compile_with_config("ui/main.slint", config.clone()).expect("编译 Slint UI 失败");
    slint_build::compile_with_config("ui/gallery.slint", config).expect("编译 Slint UI 失败");
}
