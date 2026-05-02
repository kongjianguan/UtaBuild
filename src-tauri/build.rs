fn main() {
    tauri_build::build();

    // Force rebuild when key frontend files change.
    // tauri_build emits rerun-if-changed for the frontend DIST directory (../src),
    // but cargo's directory-level watching can miss file content changes during
    // Android cross-compilation. Watching individual files is more reliable.
    println!("cargo:rerun-if-changed=../src/index.html");
    println!("cargo:rerun-if-changed=../src/js/app.js");
    println!("cargo:rerun-if-changed=../src/css/style.css");
}
