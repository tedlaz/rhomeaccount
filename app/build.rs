//! Embeds the application icon as a Win32 resource, so Explorer, the taskbar
//! pinning UI and the Alt-Tab list show it. The window icon itself is set at
//! runtime from the PNG (see `load_icon`).

fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/homeacc.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/homeacc.ico");
        // A missing resource compiler should not stop the build; the app just
        // falls back to the generic exe icon in Explorer.
        if let Err(err) = res.compile() {
            println!("cargo:warning=could not embed the app icon: {err}");
        }
    }
}
