//! Build-script helper for `tauri-plugin-google-app-functions`.
//!
//! Call it from the app's `build.rs`, after `tauri_build::build()`:
//!
//! ```ignore
//! fn main() {
//!   tauri_build::build();
//!   tauri_plugin_google_app_functions_build::Builder::new()
//!     .app_description("This app manages notes. Use `createNote` to add one.")
//!     .build();
//! }
//! ```
//!
//! It scans the crate's `src/` for `#[app_function]` functions and
//! `#[app_function_serializable]` structs and, when building for Android through the Tauri CLI,
//! writes into the Android app module:
//!
//! - `assets/generated/tauri_app_functions.xml`: the AppFunctions schema the system indexes,
//!   in the format the `androidx.appfunctions` KSP compiler produces;
//! - `assets/generated/tauri_app_functions.json`: the parameter and result types the plugin's
//!   `TauriAppFunctionService` converts calls with;
//! - `res/xml/tauri_app_functions_metadata.xml`: the app description, when one is set.
//!
//! No Kotlin, KSP or Gradle changes are needed in the app.
//!
//! Signature errors (unsupported types, duplicate names) fail the build on every target, so
//! they show up in desktop builds too.

mod android;
mod docs;
mod parse;

use std::{
  env, fs,
  path::{Path, PathBuf},
};

/// Configures the generator. See the crate docs.
#[derive(Debug, Default)]
pub struct Builder {
  app_description: Option<String>,
  source_dirs: Vec<PathBuf>,
}

impl Builder {
  pub fn new() -> Self {
    Self::default()
  }

  /// Describes the app to agents: what it does and when to use which function. Written to
  /// `res/xml/tauri_app_functions_metadata.xml` in the Android app module.
  pub fn app_description(mut self, description: impl Into<String>) -> Self {
    self.app_description = Some(description.into());
    self
  }

  /// Adds a directory (relative to the crate root) to scan for app functions.
  /// Defaults to `src` when none is given.
  pub fn source_dir(mut self, dir: impl Into<PathBuf>) -> Self {
    self.source_dirs.push(dir.into());
    self
  }

  /// Runs the generator, panicking with a readable message on error, like `tauri_build::build`.
  pub fn build(self) {
    if let Err(error) = self.try_build() {
      panic!("tauri-plugin-google-app-functions-build: {error}");
    }
  }

  pub fn try_build(self) -> Result<(), String> {
    let manifest_dir = PathBuf::from(
      env::var_os("CARGO_MANIFEST_DIR").ok_or("CARGO_MANIFEST_DIR is not set")?,
    );
    let dirs: Vec<PathBuf> = if self.source_dirs.is_empty() {
      vec![manifest_dir.join("src")]
    } else {
      self.source_dirs.iter().map(|dir| manifest_dir.join(dir)).collect()
    };
    for dir in &dirs {
      println!("cargo:rerun-if-changed={}", dir.display());
    }
    for var in [
      "TAURI_ANDROID_PROJECT_PATH",
      "WRY_ANDROID_LIBRARY",
      "WRY_ANDROID_KOTLIN_FILES_OUT_DIR",
    ] {
      println!("cargo:rerun-if-env-changed={var}");
    }

    let model = parse::parse_dirs(&dirs)
      .map_err(|errors| format!("invalid app functions:\n  {}", errors.join("\n  ")))?;

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("android") {
      return Ok(());
    }
    // Set by the Tauri CLI (`tauri android dev|build`); absent for a plain `cargo build`.
    let Some(project) = env::var_os("TAURI_ANDROID_PROJECT_PATH") else {
      println!(
        "cargo:warning=TAURI_ANDROID_PROJECT_PATH is not set; skipping AppFunctions schema generation (build through the Tauri CLI)"
      );
      return Ok(());
    };
    let library =
      env::var("WRY_ANDROID_LIBRARY").map_err(|_| "WRY_ANDROID_LIBRARY is not set")?;

    let main = PathBuf::from(project).join("app/src/main");
    let assets = main.join("assets");
    write_if_changed(&assets.join(android::SCHEMA_ASSET), &android::render_schema(&model))?;
    write_if_changed(&assets.join(android::TYPES_ASSET), &android::render_types(&model, &library))?;
    if let Some(description) = &self.app_description {
      write_if_changed(
        &main.join("res/xml/tauri_app_functions_metadata.xml"),
        &android::render_app_metadata(description),
      )?;
    }

    // Written by versions that generated Kotlin for KSP; it no longer compiles.
    if let Some(kotlin_dir) = env::var_os("WRY_ANDROID_KOTLIN_FILES_OUT_DIR") {
      let stale = PathBuf::from(kotlin_dir).join("TauriAppFunctions.kt");
      if stale.exists() {
        fs::remove_file(&stale)
          .map_err(|e| format!("failed to remove {}: {e}", stale.display()))?;
      }
    }
    Ok(())
  }
}

/// Shorthand for `Builder::new().build()`.
pub fn build() {
  Builder::new().build()
}

/// Skips unchanged files so Gradle does not recompile needlessly.
fn write_if_changed(path: &Path, content: &str) -> Result<(), String> {
  if fs::read_to_string(path).is_ok_and(|existing| existing == content) {
    return Ok(());
  }
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
  }
  fs::write(path, content).map_err(|e| format!("failed to write {}: {e}", path.display()))
}
