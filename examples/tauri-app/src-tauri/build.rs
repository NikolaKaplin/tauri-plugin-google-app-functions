fn main() {
  tauri_build::build();
  install_music_example();

  // Unit-test executables also need Common Controls v6 when linking Tauri on Windows.
  if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
    && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
  {
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!(
      "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
    );
  }

  let mut app_functions = tauri_plugin_google_app_functions_build::Builder::new().app_description(
    "Small Hours: a music player with two bundled offline tracks and a background task-store example.\n\
     Operational patterns:\n\
     - Use `playMusic`, `pauseMusic`, `nextTrack`, `previousTrack` and `playbackState` to control music without opening the UI.\n\
     - Use `createTask` to add a task the user mentions, with details in `notes`.\n\
     - Use `listTasks` to answer what is on the list; pass `includeDone = false` for open tasks only.\n\
     - Use `completeTask` or `deleteTask` with an ID from `listTasks`.",
  );
  // `source_dir` replaces the default `src`, so list both.
  if std::env::var_os("CARGO_FEATURE_TEST_FUNCTIONS").is_some() {
    app_functions = app_functions.source_dir("src").source_dir("test_functions");
  }
  app_functions.build();
}

// Keep the example's native player here rather than adding music-specific code to the plugin.
fn install_music_example() {
  use std::{env, fs, path::Path};
  println!("cargo:rerun-if-changed=android-music");
  let Some(project) = env::var_os("TAURI_ANDROID_PROJECT_PATH") else {
    return;
  };
  let main = Path::new(&project).join("app/src/main");
  let kotlin = main.join("java/com/tauri/dev/music/generated/MusicService.kt");
  let write = |path: &Path, bytes: &[u8]| {
    if fs::read(path).ok().as_deref() != Some(bytes) {
      fs::create_dir_all(path.parent().unwrap()).unwrap();
      fs::write(path, bytes).unwrap();
    }
  };
  write(&kotlin, include_bytes!("android-music/MusicService.kt"));
  for name in ["morning", "night"] {
    let bytes = fs::read(format!("android-music/assets/music/{name}.wav")).unwrap();
    write(&main.join(format!("assets/music/{name}.wav")), &bytes);
  }
  let manifest = main.join("AndroidManifest.xml");
  let mut xml = fs::read_to_string(&manifest).unwrap();
  if !xml.contains("com.tauri.dev.music.MusicService") {
    xml = xml.replace(
      "</application>",
      r#"<provider android:name="com.tauri.dev.music.MusicProvider"
            android:authorities="${applicationId}.music-init" android:exported="false" />
        <service android:name="com.tauri.dev.music.MusicService"
            android:foregroundServiceType="mediaPlayback" android:exported="false" />
    </application>"#,
    );
    xml = xml.replace(
      "<application",
      r#"<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE_MEDIA_PLAYBACK" />
    <application"#,
    );
    write(&manifest, xml.as_bytes());
  }
  write(
    &Path::new(&project).join("app/proguard-music.pro"),
    b"-keep class com.tauri.dev.music.** { *; }\n",
  );
}
