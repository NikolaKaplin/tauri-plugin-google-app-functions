fn main() {
  tauri_build::build();

  let mut app_functions = tauri_plugin_google_app_functions_build::Builder::new().app_description(
    "A todo list app.\n\
     Operational patterns:\n\
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
