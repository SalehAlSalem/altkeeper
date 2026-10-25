fn main() {
    #[cfg(windows)]
    {
        use std::env;
        use std::fs;
        use std::process::Command;

        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let out_dir = env::var("OUT_DIR").unwrap();

        let icon_path = format!("{manifest_dir}/assets/icon.ico").replace('\\', "/");
        let rc_content = format!("1 ICON \"{icon_path}\"\n");
        let rc_path = format!("{out_dir}/app.rc");
        let obj_path = format!("{out_dir}/app.o");

        fs::write(&rc_path, rc_content).expect("Failed to write app.rc");

        let status = Command::new("windres")
            .args([&rc_path, "-O", "coff", "-o", &obj_path])
            .status()
            .expect("Failed to execute windres");

        if status.success() {
            println!("cargo:rustc-link-arg-bins={obj_path}");
        }

        println!("cargo:rerun-if-changed=assets/icon.ico");
    }
}
