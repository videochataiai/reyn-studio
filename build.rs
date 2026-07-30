fn main() {
    println!("cargo:rerun-if-changed=packaging/windows/ReynStudio.ico");
    println!("cargo:rerun-if-env-changed=REYN_SKIP_WINDOWS_RESOURCES");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var_os("REYN_SKIP_WINDOWS_RESOURCES").is_none()
    {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("packaging/windows/ReynStudio.ico");
        resource
            .compile()
            .expect("compile Reyn Studio Windows icon resources");
    }
}
