mod test_functions;

use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tauri_plugin_google_app_functions::{app_function, app_function_serializable, AppFunctionError};

// Learn more about Tauri commands at https://v2.tauri.app/develop/calling-rust/#commands
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// A note saved in the app.
#[app_function_serializable]
#[derive(Clone)]
pub struct Note {
    /// Unique identifier of the note.
    pub id: i64,
    /// Short title shown in the note list.
    pub title: String,
    /// Full note text, if any.
    pub content: Option<String>,
}

#[derive(Default)]
struct Notes(Mutex<Vec<Note>>);

/// Saves a new note.
///
/// # Arguments
/// * `title` - Title of the note. Must not be empty.
/// * `content` - Optional body text.
///
/// # Returns
/// The saved note with its assigned ID.
#[app_function]
fn create_note(
    app: AppHandle,
    title: String,
    content: Option<String>,
) -> Result<Note, AppFunctionError> {
    if title.trim().is_empty() {
        return Err(AppFunctionError::InvalidArgument("title must not be empty".into()));
    }
    let notes = app.state::<Notes>();
    let mut notes = notes.0.lock().unwrap();
    let note = Note { id: notes.len() as i64 + 1, title, content };
    notes.push(note.clone());
    Ok(note)
}

/// Lists all saved notes, oldest first.
#[app_function]
fn list_notes(app: AppHandle) -> Vec<Note> {
    app.state::<Notes>().0.lock().unwrap().clone()
}

/// Counts the words in a text.
///
/// # Arguments
/// * `text` - The text to analyze.
#[app_function]
async fn count_words(text: String) -> i32 {
    text.split_whitespace().count() as i32
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Notes::default())
        .invoke_handler(tauri::generate_handler![greet])
        .plugin(tauri_plugin_google_app_functions::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
