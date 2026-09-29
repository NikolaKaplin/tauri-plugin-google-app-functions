use std::{env, fs, path::PathBuf};

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

  #[app_function]
  fn nothing() {}
}
"#;

fn setup(name: &str, source: &str) -> (PathBuf, PathBuf) {
  let root = env::temp_dir().join(format!("tpgaf-build-test-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&root);
  fs::create_dir_all(root.join("src")).unwrap();
  fs::write(root.join("src/lib.rs"), source).unwrap();
  let kotlin = root.join("android/generated");
  fs::create_dir_all(&kotlin).unwrap();
  (root, kotlin)
}

// Build scripts read their configuration from the environment, so the scenarios share one
// test to avoid racing on env vars.
#[test]
fn generates_kotlin() {
  let (root, kotlin) = setup("ok", SOURCE);
  // SAFETY: single test in this binary, nothing else reads the environment concurrently.
  unsafe {
    env::set_var("CARGO_MANIFEST_DIR", &root);
    env::set_var("CARGO_CFG_TARGET_OS", "android");
    env::set_var("WRY_ANDROID_KOTLIN_FILES_OUT_DIR", &kotlin);
    env::set_var("WRY_ANDROID_LIBRARY", "my_app_lib");
    env::set_var("TAURI_ANDROID_PROJECT_PATH", root.join("android"));
  }

  tauri_plugin_google_app_functions_build::Builder::new()
    .app_description("Manages \"tasks\" & more")
    .try_build()
    .unwrap();

  let generated = fs::read_to_string(kotlin.join("TauriAppFunctions.kt")).unwrap();
  for expected in [
    "private const val LIBRARY = \"my_app_lib\"",
    "@AppFunctionSerializable(isDescribedByKDoc = true)\ndata class Task(",
    "    /**\n     * Task ID.\n     */\n    val taskId: String,",
    "    val scores: DoubleArray,",
    "    val due: Long?,",
    "abstract class TauriAppFunctionServiceBase : AppFunctionService() {",
    "     * @param title The title.\n     * @param dueAt Due time in epoch millis.\n     * @return The created task.",
    "    suspend fun createTask(title: String, dueAt: Long? = null, `in`: List<Task>): Task {",
    "        args.put(\"in\", J.encodeList(`in`) { v1 -> encodeTask(v1) })",
    "        return decodeTask(AppFunctionsBridge.call(LIBRARY, \"create_task\", args))",
    "    suspend fun nothing() {\n        val args = JSONObject()\n        AppFunctionsBridge.call(LIBRARY, \"nothing\", args)\n    }",
    "        due = J.nullable(o.opt(\"due\")) { v1 -> J.long(v1) },",
  ] {
    assert!(generated.contains(expected), "missing:\n{expected}\n\nin:\n{generated}");
  }
  assert!(!generated.contains("app:"), "AppHandle must not be exposed:\n{generated}");

  let metadata = fs::read_to_string(
    root.join("android/app/src/main/res/xml/tauri_app_functions_metadata.xml"),
  )
  .unwrap();
  assert!(metadata.contains("appfn:description=\"Manages &quot;tasks&quot; &amp; more\""));

  let (root, _) = setup(
    "err",
    "#[app_function] fn bad(x: std::collections::HashMap<String, String>) {}",
  );
  unsafe { env::set_var("CARGO_MANIFEST_DIR", &root) };
  let error = tauri_plugin_google_app_functions_build::Builder::new().try_build().unwrap_err();
  assert!(error.contains("parameter `x`: unsupported type `HashMap`"), "{error}");
}
