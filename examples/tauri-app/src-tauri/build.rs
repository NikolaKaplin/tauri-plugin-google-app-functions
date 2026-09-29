fn main() {
  tauri_build::build();
  tauri_plugin_google_app_functions_build::Builder::new()
    .app_description(
      "A note-taking app.\n\
       Operational patterns:\n\
       - Use `createNote` to save a note, `listNotes` to read saved notes.\n\
       - Use `countWords` to count words in any text; it works even when the app is closed.",
    )
    .build();
}
