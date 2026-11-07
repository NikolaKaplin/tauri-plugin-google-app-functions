mod music;
mod tasks;
#[cfg(feature = "test-functions")]
#[path = "../test_functions/mod.rs"]
mod test_functions;

use std::sync::Arc;

use tauri::{Manager, State};
use tauri_plugin_google_app_functions::{AppFunctionError, app_function};

use tasks::{Task, TaskStore};

// ----- App functions: what Gemini and other agents can call -----

/// Adds a task to the user's todo list.
///
/// # Arguments
/// * `title` - What needs to be done, in a few words. Must not be empty.
/// * `notes` - Extra details, such as a place, a time or a list of items.
///
/// # Returns
/// The new task with its ID.
#[app_function]
fn create_task(title: String, notes: Option<String>) -> Result<Task, AppFunctionError> {
  tasks::store()?.add(title, notes, true)
}

/// Lists the tasks in the user's todo list, oldest first.
///
/// # Arguments
/// * `include_done` - Whether to include finished tasks. Defaults to true.
#[app_function]
fn list_tasks(include_done: Option<bool>) -> Result<Vec<Task>, AppFunctionError> {
  let tasks = tasks::store()?.all();
  Ok(match include_done {
    Some(false) => tasks.into_iter().filter(|t| !t.done).collect(),
    _ => tasks,
  })
}

/// Returns one task of the todo list.
///
/// # Arguments
/// * `id` - ID of the task, as returned by `listTasks` or `createTask`.
#[app_function]
fn get_task(id: i64) -> Result<Task, AppFunctionError> {
  tasks::store()?.get(id)
}

/// Marks a task as done.
///
/// # Arguments
/// * `id` - ID of the task to finish.
///
/// # Returns
/// The updated task.
#[app_function]
fn complete_task(id: i64) -> Result<Task, AppFunctionError> {
  tasks::store()?.set_done(id, true)
}

/// Deletes a task from the todo list.
///
/// # Arguments
/// * `id` - ID of the task to delete.
#[app_function]
fn delete_task(id: i64) -> Result<(), AppFunctionError> {
  tasks::store()?.remove(id)
}

// ----- Commands: what the app's own UI calls -----

#[tauri::command]
fn tasks(store: State<Arc<TaskStore>>) -> Vec<Task> {
  store.all()
}

#[tauri::command]
fn add_task(store: State<Arc<TaskStore>>, title: String) -> Result<Task, AppFunctionError> {
  store.add(title, None, false)
}

#[tauri::command]
fn set_task_done(
  store: State<Arc<TaskStore>>,
  id: i64,
  done: bool,
) -> Result<Task, AppFunctionError> {
  store.set_done(id, done)
}

#[tauri::command]
fn remove_task(store: State<Arc<TaskStore>>, id: i64) -> Result<(), AppFunctionError> {
  store.remove(id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_google_app_functions::init())
    .setup(|app| {
      let store = tasks::initialize(app.path().app_data_dir()?)?;
      app.manage(store);
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      tasks,
      add_task,
      set_task_done,
      remove_task,
      music::music_command
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
