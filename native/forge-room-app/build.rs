// Embeds the Forge Room icon and version info into the Windows executable so Explorer,
// the Start menu, the taskbar and shortcuts show the logo instead of a generic icon.
fn main() {
    println!("cargo:rerun-if-changed=assets/forge-room.ico");
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/forge-room.ico");
        resource.set("ProductName", "Forge Room Early Preview");
        resource.set("FileDescription", "Forge Room Early Preview");
        resource
            .compile()
            .expect("failed to embed the Windows icon resource");
    }
}
