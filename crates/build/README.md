# tauri-plugin-google-app-functions-build

`build.rs` helper for
[tauri-plugin-google-app-functions](https://crates.io/crates/tauri-plugin-google-app-functions).

It scans the app's `src/` for `#[app_function]` functions and `#[app_function_serializable]`
structs and writes the Android AppFunctions schema into the Tauri Android project, so the app
needs no Kotlin, KSP or Gradle changes.

```toml
# src-tauri/Cargo.toml
[build-dependencies]
tauri-plugin-google-app-functions-build = "0.1"
```

```rust
// src-tauri/build.rs
fn main() {
    tauri_build::build();
    tauri_plugin_google_app_functions_build::Builder::new()
        .app_description("A note-taking app. Use `createNote` to save a note.")
        .build();
}
```

See the [plugin's README](https://github.com/NikolaKaplin/tauri-plugin-google-app-functions#readme)
for the full setup.

## License

[MIT](https://github.com/NikolaKaplin/tauri-plugin-google-app-functions/blob/main/LICENSE)
