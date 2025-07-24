use std::env;

fn main() {
    // Set necessary environment variables for WebKit to prevent initialization errors
    println!("cargo:rustc-env=WEBKIT_DISABLE_DMABUF_RENDERER=1");
    
    tauri_build::build()
}
