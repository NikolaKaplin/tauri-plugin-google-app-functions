use tauri::{
  plugin::{Builder, TauriPlugin},
  Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::GoogleAppFunctions;
#[cfg(mobile)]
use mobile::GoogleAppFunctions;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the google-app-functions APIs.
pub trait GoogleAppFunctionsExt<R: Runtime> {
  fn google_app_functions(&self) -> &GoogleAppFunctions<R>;
}

impl<R: Runtime, T: Manager<R>> crate::GoogleAppFunctionsExt<R> for T {
  fn google_app_functions(&self) -> &GoogleAppFunctions<R> {
    self.state::<GoogleAppFunctions<R>>().inner()
  }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("google-app-functions")
    .invoke_handler(tauri::generate_handler![commands::ping])
    .setup(|app, api| {
      #[cfg(mobile)]
      let google_app_functions = mobile::init(app, api)?;
      #[cfg(desktop)]
      let google_app_functions = desktop::init(app, api)?;
      app.manage(google_app_functions);
      Ok(())
    })
    .build()
}
