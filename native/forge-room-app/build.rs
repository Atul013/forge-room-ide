// Embeds the Forge Room icon and version info into the Windows executable so Explorer,
// the Start menu, the taskbar and shortcuts show the logo instead of a generic icon.
fn main() {
    for name in ["FORGE_ROOM_BUILD_SOURCE", "FORGE_ROOM_BUILD_NATIVE_TREE"] {
        println!("cargo:rerun-if-env-changed={name}");
        let value = std::env::var(name).unwrap_or_default();
        assert!(
            value.is_empty()
                || [40, 64].contains(&value.len()) && value.bytes().all(|b| b.is_ascii_hexdigit()),
            "Invalid recorded build identity: {name}"
        );
        println!("cargo:rustc-env={name}={value}");
    }
    println!(
        "cargo:rustc-env=FORGE_ROOM_BUILD_TARGET={}",
        std::env::var("TARGET").expect("Cargo target")
    );
    println!(
        "cargo:rustc-env=FORGE_ROOM_BUILD_PROFILE={}",
        std::env::var("PROFILE").expect("Cargo profile")
    );
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
