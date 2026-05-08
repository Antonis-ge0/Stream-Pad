fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=installer.rc");
        println!("cargo:rerun-if-changed=..\\src-tauri\\icons\\icon.ico");
        embed_resource::compile("installer.rc", embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
