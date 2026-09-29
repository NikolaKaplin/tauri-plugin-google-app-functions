use serde_json::json;
use tauri_plugin_google_app_functions::{
  app_function, app_function_serializable, invoke, registered_functions, AppFunctionError,
};

/// A task.
#[app_function_serializable]
#[derive(Debug, Clone)]
pub struct Task {
  /// Task ID.
  pub task_id: String,
  /// Optional tags.
  pub tags: Vec<String>,
  pub due: Option<i64>,
}

/// Creates a task.
///
/// # Arguments
/// * `title` - The title.
#[app_function]
async fn create_task(title: String, due_at: Option<i64>) -> Result<Task, AppFunctionError> {
  if title.is_empty() {
    return Err(AppFunctionError::InvalidArgument("title is empty".into()));
  }
  Ok(Task { task_id: title, tags: vec!["new".into()], due: due_at })
}

#[app_function]
fn add(a: i32, b: i32) -> i32 {
  a + b
}

#[app_function]
fn nothing() {}

#[app_function]
fn needs_app(app: tauri::AppHandle) -> String {
  app.package_info().name.clone()
}

#[test]
fn registers_all_functions() {
  let mut names: Vec<_> = registered_functions().collect();
  names.sort();
  assert_eq!(names, ["add", "create_task", "needs_app", "nothing"]);
}

#[test]
fn invokes_with_camel_case_json() {
  tauri::async_runtime::block_on(async {
    let task = invoke("create_task", json!({ "title": "t", "dueAt": 5 })).await.unwrap();
    assert_eq!(task, json!({ "taskId": "t", "tags": ["new"], "due": 5 }));

    let task = invoke("create_task", json!({ "title": "t", "dueAt": null })).await.unwrap();
    assert_eq!(task["due"], json!(null));

    assert_eq!(invoke("add", json!({ "a": 2, "b": 3 })).await.unwrap(), json!(5));
    assert_eq!(invoke("nothing", json!({})).await.unwrap(), json!(null));
  });
}

#[test]
fn maps_errors() {
  tauri::async_runtime::block_on(async {
    let err = invoke("create_task", json!({ "title": "" })).await.unwrap_err();
    assert!(matches!(err, AppFunctionError::InvalidArgument(_)));

    let err = invoke("add", json!({ "a": "x" })).await.unwrap_err();
    assert!(matches!(err, AppFunctionError::InvalidArgument(_)));

    let err = invoke("missing", json!({})).await.unwrap_err();
    assert!(matches!(err, AppFunctionError::FunctionNotFound(_)));

    // The plugin was never initialized, so there is no app handle to inject.
    let err = invoke("needs_app", json!({})).await.unwrap_err();
    assert!(matches!(err, AppFunctionError::Unknown(_)));

    assert_eq!(
      serde_json::to_value(AppFunctionError::NotSupported("x".into())).unwrap(),
      json!({ "kind": "notSupported", "message": "x" })
    );
  });
}
