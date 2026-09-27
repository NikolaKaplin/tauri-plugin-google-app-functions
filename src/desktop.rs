use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  _api: PluginApi<R, C>,
) -> crate::Result<GoogleAppFunctions<R>> {
  Ok(GoogleAppFunctions(app.clone()))
}

/// Access to the google-app-functions APIs.
pub struct GoogleAppFunctions<R: Runtime>(AppHandle<R>);

impl<R: Runtime> GoogleAppFunctions<R> {
  pub fn ping(&self, payload: PingRequest) -> crate::Result<PingResponse> {
    Ok(PingResponse {
      value: payload.value,
    })
  }
}
