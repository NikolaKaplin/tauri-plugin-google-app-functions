use std::{
  env, fs,
  path::{Path, PathBuf},
};

const SOURCE: &str = r#"
/// A task.
#[app_function_serializable]
pub struct Task {
  /// Task ID.
  pub task_id: String,
  pub tags: Vec<String>,
  pub scores: Vec<f64>,
  pub due: Option<i64>,
}

/// Not used by any function.
#[app_function_serializable]
pub struct Unused {
  pub x: i32,
}

mod inner {
  /// Creates a task.
  ///
  /// # Arguments
  /// * `title` - The title.
  /// * `due_at` - Due time in epoch millis.
  ///
  /// # Returns
  /// The created task.
  #[tauri_plugin_google_app_functions::app_function]
  async fn create_task(app: tauri::AppHandle, title: String, due_at: Option<i64>, r#in: Vec<Task>) -> Result<Task, AppFunctionError> {
    todo!()
  }

  /// Escapes <xml> & "json".
  #[app_function]
  fn nothing() {}
}
"#;

const KSP_PREFIX: &str = "com.plugin.google_app_functions.generated.";
const KSP_SERVICE: &str = "com.plugin.google_app_functions.generated.TauriAppFunctionServiceBase";

fn setup(name: &str, source: &str) -> PathBuf {
  let root = env::temp_dir().join(format!("tpgaf-build-test-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&root);
  fs::create_dir_all(root.join("src")).unwrap();
  fs::write(root.join("src/lib.rs"), source).unwrap();
  root
}

fn read(path: &Path) -> String {
  fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

// Build scripts read their configuration from the environment, so the scenarios share one
// test to avoid racing on env vars.
#[test]
fn generates_android_files() {
  let root = setup("ok", SOURCE);
  let project = root.join("android");
  let kotlin = project.join("app/src/main/java/com/example/generated");
  fs::create_dir_all(&kotlin).unwrap();
  fs::write(kotlin.join("TauriAppFunctions.kt"), "stale").unwrap();
  // SAFETY: single test in this binary, nothing else reads the environment concurrently.
  unsafe {
    env::set_var("CARGO_MANIFEST_DIR", &root);
    env::set_var("CARGO_CFG_TARGET_OS", "android");
    env::set_var("TAURI_ANDROID_PROJECT_PATH", &project);
    env::set_var("WRY_ANDROID_LIBRARY", "my_app_lib");
    env::set_var("WRY_ANDROID_KOTLIN_FILES_OUT_DIR", &kotlin);
  }

  tauri_plugin_google_app_functions_build::Builder::new()
    .app_description("Manages \"tasks\" & more")
    .try_build()
    .unwrap();

  let assets = project.join("app/src/main/assets/generated");
  let schema = read(&assets.join("tauri_app_functions.xml"));
  for expected in [
    "        <id>com.plugin.google_app_functions.TauriAppFunctionService#createTask</id>",
    "            <name>dueAt</name>\n            <description>Due time in epoch millis.</description>",
    "            <name>in</name>",
    "        <description>Escapes &lt;xml&gt; &amp; \"json\".</description>",
    "            <description>The created task.</description>",
    "            <name>com.plugin.google_app_functions.Task</name>",
  ] {
    assert!(schema.contains(expected), "missing:\n{expected}\n\nin:\n{schema}");
  }
  assert!(!schema.contains("Unused"), "unused structs stay out of the schema:\n{schema}");
  assert!(!schema.contains("<name>app</name>"), "AppHandle must not be exposed:\n{schema}");

  let types = read(&assets.join("tauri_app_functions.json"));
  for expected in [
    "\"library\": \"my_app_lib\"",
    "\"com.plugin.google_app_functions.TauriAppFunctionService#createTask\": {\"name\": \"create_task\", \"params\": [[\"title\", \"string\"], [\"dueAt\", \"long?\"], [\"in\", \"@com.plugin.google_app_functions.Task[]\"]], \"returns\": \"@com.plugin.google_app_functions.Task\"}",
    "\"com.plugin.google_app_functions.TauriAppFunctionService#nothing\": {\"name\": \"nothing\", \"params\": [], \"returns\": \"unit\"}",
    "\"com.plugin.google_app_functions.Task\": [[\"taskId\", \"string\"], [\"tags\", \"string[]\"], [\"scores\", \"double[]\"], [\"due\", \"long?\"]]",
  ] {
    assert!(types.contains(expected), "missing:\n{expected}\n\nin:\n{types}");
  }

  let metadata = read(&project.join("app/src/main/res/xml/tauri_app_functions_metadata.xml"));
  assert!(metadata.contains("appfn:description=\"Manages &quot;tasks&quot; &amp; more\""));
  assert!(!kotlin.join("TauriAppFunctions.kt").exists(), "stale KSP input must be removed");

  // The schema of the example app matches, byte for byte, what the androidx.appfunctions KSP
  // compiler produced for the equivalent Kotlin.
  let example_src =
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/tauri-app/src-tauri/src");
  tauri_plugin_google_app_functions_build::Builder::new()
    .source_dir(&example_src)
    .try_build()
    .unwrap();
  let expected = read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ksp_tauri_app_functions.xml").as_path())
    .replace("\r\n", "\n")
    .replace(KSP_SERVICE, "com.plugin.google_app_functions.TauriAppFunctionService")
    .replace(KSP_PREFIX, "com.plugin.google_app_functions.");
  let actual = read(&assets.join("tauri_app_functions.xml"));
  assert!(actual == expected, "schema differs from KSP output:\n{}", first_difference(&expected, &actual));

  let root = setup("err", "#[app_function] fn bad(x: std::collections::HashMap<String, String>) {}");
  unsafe { env::set_var("CARGO_MANIFEST_DIR", &root) };
  let error = tauri_plugin_google_app_functions_build::Builder::new().try_build().unwrap_err();
  assert!(error.contains("parameter `x`: unsupported type `HashMap`"), "{error}");
}

fn first_difference(expected: &str, actual: &str) -> String {
  for (i, (e, a)) in expected.lines().zip(actual.lines()).enumerate() {
    if e != a {
      return format!("line {}:\n  expected: {e}\n  actual:   {a}", i + 1);
    }
  }
  format!(
    "line counts differ: expected {}, actual {}",
    expected.lines().count(),
    actual.lines().count()
  )
}
