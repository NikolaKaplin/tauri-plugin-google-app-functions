//! Persistent state shared by UI commands and cold-started AppFunctions.
//! Initializing this store does not start Tauri, an Activity, or a WebView.

use std::{
  fs,
  path::PathBuf,
  sync::{Arc, Mutex},
};
use tauri::Emitter;
#[cfg(not(target_os = "android"))]
use tauri::Manager;
use tauri_plugin_google_app_functions::{AppFunctionError, app_function_serializable};

/// A task in the todo list.
#[app_function_serializable]
#[derive(Clone)]
pub struct Task {
  /// Unique identifier of the task.
  pub id: i64,
  /// What needs to be done, in a few words.
  pub title: String,
  /// Extra details, if any.
  pub notes: Option<String>,
  /// Whether the task is done.
  pub done: bool,
  /// Whether an agent created the task through App Functions rather than the UI.
  pub from_agent: bool,
}

pub struct TaskStore {
  tasks: Mutex<Vec<Task>>,
  file: PathBuf,
}

static STORE: Mutex<Option<Arc<TaskStore>>> = Mutex::new(None);

/// Both entry points reuse the same store, regardless of which starts first.
pub fn initialize(directory: PathBuf) -> Result<Arc<TaskStore>, AppFunctionError> {
  let mut shared = STORE.lock().unwrap();
  if let Some(store) = shared.as_ref() {
    return Ok(store.clone());
  }
  let store = Arc::new(TaskStore::open(directory.join("tasks.json"))?);
  *shared = Some(store.clone());
  Ok(store)
}

/// Called from AppFunctions without requiring an AppHandle.
pub fn store() -> Result<Arc<TaskStore>, AppFunctionError> {
  if let Some(store) = STORE.lock().unwrap().as_ref() {
    return Ok(store.clone());
  }
  #[cfg(target_os = "android")]
  let directory = tauri_plugin_google_app_functions::android_app_data_dir()?;
  #[cfg(not(target_os = "android"))]
  let directory = tauri_plugin_google_app_functions::app_handle::<tauri::Wry>()?
    .path()
    .app_data_dir()
    .map_err(storage_error)?;
  initialize(directory)
}

fn sample_tasks() -> Vec<Task> {
  [
    (
      "Try the demo with adb",
      Some("See the README of the example"),
    ),
    ("Ask Gemini to add a task", None),
    ("Star tauri-plugin-google-app-functions", None),
  ]
  .into_iter()
  .enumerate()
  .map(|(i, (title, notes))| Task {
    id: i as i64 + 1,
    title: title.into(),
    notes: notes.map(str::to_owned),
    done: false,
    from_agent: false,
  })
  .collect()
}

impl TaskStore {
  fn open(file: PathBuf) -> Result<Self, AppFunctionError> {
    let tasks = match fs::read_to_string(&file) {
      Ok(json) => serde_json::from_str(&json).map_err(storage_error)?,
      Err(error) if error.kind() == std::io::ErrorKind::NotFound => sample_tasks(),
      Err(error) => return Err(storage_error(error)),
    };
    Ok(Self {
      tasks: Mutex::new(tasks),
      file,
    })
  }

  pub fn all(&self) -> Vec<Task> {
    self.tasks.lock().unwrap().clone()
  }

  pub fn get(&self, id: i64) -> Result<Task, AppFunctionError> {
    self
      .all()
      .into_iter()
      .find(|task| task.id == id)
      .ok_or_else(|| not_found(id))
  }

  pub fn add(
    &self,
    title: String,
    notes: Option<String>,
    from_agent: bool,
  ) -> Result<Task, AppFunctionError> {
    let title = title.trim().to_owned();
    if title.is_empty() {
      return Err(AppFunctionError::InvalidArgument(
        "title must not be empty".into(),
      ));
    }
    let notes = notes.map(|n| n.trim().to_owned()).filter(|n| !n.is_empty());
    self.change(|tasks| {
      let id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
      let task = Task {
        id,
        title,
        notes,
        done: false,
        from_agent,
      };
      tasks.push(task.clone());
      Ok(task)
    })
  }

  pub fn set_done(&self, id: i64, done: bool) -> Result<Task, AppFunctionError> {
    self.change(|tasks| {
      let task = tasks
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(|| not_found(id))?;
      task.done = done;
      Ok(task.clone())
    })
  }

  pub fn remove(&self, id: i64) -> Result<(), AppFunctionError> {
    self.change(|tasks| {
      let index = tasks
        .iter()
        .position(|t| t.id == id)
        .ok_or_else(|| not_found(id))?;
      tasks.remove(index);
      Ok(())
    })
  }

  fn change<T>(
    &self,
    update: impl FnOnce(&mut Vec<Task>) -> Result<T, AppFunctionError>,
  ) -> Result<T, AppFunctionError> {
    // Keep mutation and persistence in one critical section: UI and service share a process.
    let mut tasks = self.tasks.lock().unwrap();
    let mut snapshot = tasks.clone();
    let result = update(&mut snapshot)?;
    if let Some(parent) = self.file.parent() {
      fs::create_dir_all(parent).map_err(storage_error)?;
    }
    fs::write(
      &self.file,
      serde_json::to_vec_pretty(&snapshot).map_err(storage_error)?,
    )
    .map_err(storage_error)?;
    *tasks = snapshot.clone();
    // A live UI is optional. Cold service starts never initialize Tauri.
    if let Ok(app) = tauri_plugin_google_app_functions::app_handle::<tauri::Wry>() {
      let _ = app.emit("tasks-changed", &snapshot);
    }
    Ok(result)
  }
}

fn storage_error(error: impl std::fmt::Display) -> AppFunctionError {
  AppFunctionError::Unknown(format!("task storage: {error}"))
}

fn not_found(id: i64) -> AppFunctionError {
  AppFunctionError::ElementNotFound(format!("no task with id {id}"))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn background_changes_survive_a_new_store() {
    let directory =
      std::env::temp_dir().join(format!("tauri-todo-background-{}", std::process::id()));
    let file = directory.join("tasks.json");
    let store = TaskStore::open(file.clone()).unwrap();
    let task = store
      .add("Background task".into(), Some("No WebView".into()), true)
      .unwrap();
    store.set_done(task.id, true).unwrap();
    let reopened = TaskStore::open(file).unwrap();
    let restored = reopened.get(task.id).unwrap();
    assert!(restored.done && restored.from_agent);
    assert_eq!(restored.notes.as_deref(), Some("No WebView"));
    reopened.remove(task.id).unwrap();
    assert!(matches!(
      reopened.get(task.id),
      Err(AppFunctionError::ElementNotFound(_))
    ));
    fs::remove_file(directory.join("tasks.json")).unwrap();
    fs::remove_dir(directory).unwrap();
  }

  #[test]
  fn failed_save_does_not_change_memory() {
    let directory =
      std::env::temp_dir().join(format!("tauri-todo-save-failure-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let file = directory.join("tasks.json");
    let store = TaskStore::open(file.clone()).unwrap();
    fs::create_dir(&file).unwrap();
    let count = store.all().len();
    assert!(store.add("Unsaved".into(), None, true).is_err());
    assert_eq!(store.all().len(), count);
    fs::remove_dir(file).unwrap();
    fs::remove_dir(directory).unwrap();
  }
}
