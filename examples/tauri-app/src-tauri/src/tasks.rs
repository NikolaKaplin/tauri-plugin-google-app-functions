//! The todo list: stored in a JSON file in the app's data directory, shared by the UI commands
//! and the app functions. Every change is broadcast to the UI as a `tasks-changed` event, so a
//! task an agent adds shows up on screen right away.

use std::{fs, path::PathBuf, sync::Mutex};

use tauri::{AppHandle, Emitter, Manager, Runtime};
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
  /// Whether an agent created the task through App Functions rather than the user in the app.
  pub from_agent: bool,
}

#[derive(Default)]
pub struct TaskStore {
  tasks: Mutex<Vec<Task>>,
  file: Mutex<Option<PathBuf>>,
}

/// Seed shown on the first launch, so the list is not empty in screenshots.
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
    title: title.to_owned(),
    notes: notes.map(str::to_owned),
    done: false,
    from_agent: false,
  })
  .collect()
}

impl TaskStore {
  /// Loads the saved tasks, or the sample tasks on the first launch.
  pub fn load<R: Runtime>(&self, app: &AppHandle<R>) -> tauri::Result<()> {
    let file = app.path().app_data_dir()?.join("tasks.json");
    let tasks = fs::read_to_string(&file)
      .ok()
      .and_then(|json| serde_json::from_str(&json).ok())
      .unwrap_or_else(sample_tasks);
    *self.tasks.lock().unwrap() = tasks;
    *self.file.lock().unwrap() = Some(file);
    Ok(())
  }

  pub fn all(&self) -> Vec<Task> {
    self.tasks.lock().unwrap().clone()
  }

  pub fn get(&self, id: i64) -> Result<Task, AppFunctionError> {
    self
      .all()
      .into_iter()
      .find(|t| t.id == id)
      .ok_or_else(|| not_found(id))
  }

  pub fn add<R: Runtime>(
    &self,
    app: &AppHandle<R>,
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
    self.change(app, |tasks| {
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

  pub fn set_done<R: Runtime>(
    &self,
    app: &AppHandle<R>,
    id: i64,
    done: bool,
  ) -> Result<Task, AppFunctionError> {
    self.change(app, |tasks| {
      let task = tasks
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(|| not_found(id))?;
      task.done = done;
      Ok(task.clone())
    })
  }

  pub fn remove<R: Runtime>(&self, app: &AppHandle<R>, id: i64) -> Result<(), AppFunctionError> {
    self.change(app, |tasks| {
      let index = tasks
        .iter()
        .position(|t| t.id == id)
        .ok_or_else(|| not_found(id))?;
      tasks.remove(index);
      Ok(())
    })
  }

  /// Applies `update`, then saves the list and tells the UI.
  fn change<R: Runtime, T>(
    &self,
    app: &AppHandle<R>,
    update: impl FnOnce(&mut Vec<Task>) -> Result<T, AppFunctionError>,
  ) -> Result<T, AppFunctionError> {
    let (result, snapshot) = {
      let mut tasks = self.tasks.lock().unwrap();
      let result = update(&mut tasks)?;
      (result, tasks.clone())
    };
    if let Some(file) = self.file.lock().unwrap().as_ref() {
      let saved = file
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::write(file, serde_json::to_string_pretty(&snapshot).unwrap()));
      if let Err(e) = saved {
        return Err(AppFunctionError::Unknown(format!(
          "failed to save tasks: {e}"
        )));
      }
    }
    let _ = app.emit("tasks-changed", &snapshot);
    Ok(result)
  }
}

fn not_found(id: i64) -> AppFunctionError {
  AppFunctionError::ElementNotFound(format!("no task with id {id}"))
}
