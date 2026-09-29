mod tasks;
#[cfg(feature = "test-functions")]
#[path = "../test_functions/mod.rs"]
mod test_functions;

use tauri::{AppHandle, Manager, State};
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
fn create_task(
  app: AppHandle,
  title: String,
  notes: Option<String>,
) -> Result<Task, AppFunctionError> {
  app.state::<TaskStore>().add(&app, title, notes, true)
}

/// Lists the tasks in the user's todo list, oldest first.
///
/// # Arguments
/// * `include_done` - Whether to include finished tasks. Defaults to true.
#[app_function]
fn list_tasks(app: AppHandle, include_done: Option<bool>) -> Vec<Task> {
  let tasks = app.state::<TaskStore>().all();
  match include_done {
    Some(false) => tasks.into_iter().filter(|t| !t.done).collect(),
    _ => tasks,
  }
}

/// Returns one task of the todo list.
///
/// # Arguments
/// * `id` - ID of the task, as returned by `listTasks` or `createTask`.
#[app_function]
fn get_task(app: AppHandle, id: i64) -> Result<Task, AppFunctionError> {
  app.state::<TaskStore>().get(id)
}

/// Marks a task as done.
///
/// # Arguments
/// * `id` - ID of the task to finish.
///
/// # Returns
/// The updated task.
#[app_function]
fn complete_task(app: AppHandle, id: i64) -> Result<Task, AppFunctionError> {
  app.state::<TaskStore>().set_done(&app, id, true)
}

/// Deletes a task from the todo list.
///
/// # Arguments
/// * `id` - ID of the task to delete.
#[app_function]
fn delete_task(app: AppHandle, id: i64) -> Result<(), AppFunctionError> {
  app.state::<TaskStore>().remove(&app, id)
}

// ----- Commands: what the app's own UI calls -----

#[tauri::command]
fn tasks(store: State<TaskStore>) -> Vec<Task> {
  store.all()
}

#[tauri::command]
fn add_task(
  app: AppHandle,
  store: State<TaskStore>,
  title: String,
) -> Result<Task, AppFunctionError> {
  store.add(&app, title, None, false)
}

#[tauri::command]
fn set_task_done(
  app: AppHandle,
  store: State<TaskStore>,
  id: i64,
  done: bool,
) -> Result<Task, AppFunctionError> {
  store.set_done(&app, id, done)
}

#[tauri::command]
fn remove_task(app: AppHandle, store: State<TaskStore>, id: i64) -> Result<(), AppFunctionError> {
  store.remove(&app, id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_google_app_functions::init())
    .manage(TaskStore::default())
    .setup(|app| {
      app.state::<TaskStore>().load(app.handle())?;
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      tasks,
      add_task,
      set_task_done,
      remove_task
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
