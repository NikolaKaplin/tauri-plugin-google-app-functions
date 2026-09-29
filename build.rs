// The plugin has no commands: agents call app functions through Android, not the webview.
const COMMANDS: &[&str] = &[];

fn main() {
  tauri_plugin::Builder::new(COMMANDS).android_path("android").build();

  // Test binaries link tauri, which needs Common Controls v6 on Windows; without this manifest
  // they exit with STATUS_ENTRYPOINT_NOT_FOUND.
  let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
  let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
  if target_os == "windows" && target_env == "msvc" {
    println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    println!(
      "cargo:rustc-link-arg-tests=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
    );
  }
}
