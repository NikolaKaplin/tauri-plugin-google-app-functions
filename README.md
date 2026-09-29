# Tauri Plugin google-app-functions

Expose Rust functions of a Tauri app as Android
[AppFunctions](https://developer.android.com/ai/appfunctions), so Gemini and other on-device
agents can discover and call them. Write the function once in Rust; the plugin generates the
Kotlin `@AppFunction` service for you.

```rust
use tauri::{AppHandle, Manager};
use tauri_plugin_google_app_functions::{app_function, app_function_serializable, AppFunctionError};

/// A note saved in the app.
#[app_function_serializable]
pub struct Note {
    /// Unique identifier of the note.
    pub id: i64,
    /// Short title shown in the note list.
    pub title: String,
    /// Full note text, if any.
    pub content: Option<String>,
}

/// Saves a new note.
///
/// # Arguments
/// * `title` - Title of the note. Must not be empty.
/// * `content` - Optional body text.
///
/// # Returns
/// The saved note with its assigned ID.
#[app_function]
async fn create_note(
    app: AppHandle,
    title: String,
    content: Option<String>,
) -> Result<Note, AppFunctionError> {
    // ...
}
```

Android devices on API 36+ index the function as
`com.plugin.google_app_functions.generated.TauriAppFunctionServiceBase#createNote`, with the
doc comments as its description.

## How it works

```
agent ──► TauriAppFunctionService (Kotlin, generated) ──JNI──► #[app_function] fn (Rust)
```

1. `#[app_function]` registers the function in a global registry, keyed by its Rust name.
2. `tauri-plugin-google-app-functions-build`, called from the app's `build.rs`, scans `src/`
   and writes `TauriAppFunctions.kt` into the Android project: one `@AppFunction` method per
   Rust function and one `@AppFunctionSerializable` data class per serializable struct.
3. KSP turns that Kotlin into the AppFunctions XML schema and the concrete service. The plugin's
   `AndroidManifest.xml` already registers the service.
4. At runtime each call goes to Rust as JSON and runs on Tauri's async runtime. The result or
   error goes back the same way.

Android can start the service without opening the app. The Rust library is then loaded, but the
Tauri app is not built. Functions without an `AppHandle` parameter still work. Functions that
take one fail with `AppFunctionAppUnknownException` until the user opens the app.

## Setup

Requires Tauri 2.12+, `compileSdk`/`targetSdk` 37 (AppFunctions needs 36+), and `minSdk` 24+.

**`src-tauri/Cargo.toml`**

```toml
[dependencies]
tauri-plugin-google-app-functions = "0.1"

[build-dependencies]
tauri-plugin-google-app-functions-build = "0.1"
```

**`src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build();
    tauri_plugin_google_app_functions_build::Builder::new()
        // Optional: tells agents what the app is for and when to use which function.
        .app_description("A note-taking app. Use `createNote` to save a note.")
        .build();
}
```

**`src-tauri/src/lib.rs`**: register the plugin. It supplies the `AppHandle` that app functions
receive.

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_google_app_functions::init())
```

**`src-tauri/gen/android/build.gradle.kts`**: put KSP on the buildscript classpath. Its version
must match the Kotlin version: `2.2.10` in the Tauri 2.12 template.

```kotlin
buildscript {
    dependencies {
        classpath("com.google.devtools.ksp:com.google.devtools.ksp.gradle.plugin:2.2.10-2.0.2")
    }
}
```

**`src-tauri/gen/android/app/build.gradle.kts`**

```kotlin
plugins {
    // ...
    id("com.google.devtools.ksp")
}

dependencies {
    ksp("androidx.appfunctions:appfunctions-compiler:1.0.0-alpha12")
}

ksp {
    arg("appfunctions:aggregateAppFunctions", "true")
}

// The Rust build (build.rs) writes the AppFunctions Kotlin sources, so KSP must run after it.
tasks.matching { it.name.startsWith("ksp") }.configureEach {
    mustRunAfter(tasks.matching { it.name.startsWith("rustBuild") })
}
```

See [examples/tauri-app](examples/tauri-app) for a complete app.

## Writing app functions

`#[app_function]` works on free functions, sync or `async`. Parameters and return values may use:

| Rust | Kotlin |
| --- | --- |
| `bool` | `Boolean` |
| `i8`, `i16`, `i32`, `u8`, `u16` | `Int` |
| `i64`, `u32`, `u64`, `isize`, `usize` | `Long` |
| `f32` / `f64` | `Float` / `Double` |
| `String` | `String` |
| `Option<T>` | `T?`; optional parameters default to `null` |
| `Vec<primitive>` | `IntArray`, `LongArray`, … |
| `Vec<T>` | `List<T>` |
| struct with `#[app_function_serializable]` | `@AppFunctionSerializable data class` |
| `()` (return only) | `Unit` |

- **Names.** `snake_case` becomes `camelCase`, both for function names and field names. The
  JSON exchanged with Kotlin uses the camelCase names as well.
- **Injected parameters.** A parameter of type `tauri::AppHandle` is filled in by the plugin and
  is not visible to agents. Use it to reach managed state: `app.state::<T>()`.
- **Errors.** Return `Result<T, E>` where `E: Into<AppFunctionError>`. Each variant maps to the
  matching `androidx.appfunctions` exception: `InvalidArgument`, `ElementNotFound`,
  `PermissionRequired`, and so on. `String`, `&str` and `tauri::Error` convert to `Unknown`.
- **Descriptions.** Agents read the doc comments, so write them for an LLM:
  - Parameters go in a `# Arguments` section as `` * `name` - description `` bullets.
  - The return value goes in a `# Returns` section.
  - Struct field docs become per-property KDoc.
- **Serializable structs.** `#[app_function_serializable]` derives `Serialize` and `Deserialize`
  with `rename_all = "camelCase"`. Don't derive serde yourself, and don't add serde rename
  attributes: the Kotlin side would not see them.
- **Unique names.** Function and struct names must be unique across the crate. The generator
  finds items by parsing `src/`, so items produced by other macros or hidden behind `cfg` still
  count toward that check.

Unsupported types and duplicate names fail the build with a message on every target, desktop
included.

### Testing without a device

Registered functions can be called directly, using the same JSON Android sends:

```rust
let note = tauri_plugin_google_app_functions::invoke(
    "create_note",
    serde_json::json!({ "title": "Groceries", "content": null }),
).await?;
```

### Testing on a device

```sh
adb shell cmd app_function list-app-functions
adb shell "cmd app_function execute-app-function --package com.tauri.dev \
  --function 'com.plugin.google_app_functions.generated.TauriAppFunctionServiceBase#countWords' \
  --parameters '{\"text\": \"hello app functions\"}'"
```
