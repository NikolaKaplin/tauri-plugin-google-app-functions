//! AppFunctions and UI share the same native player. No AppHandle is needed.
use tauri_plugin_google_app_functions::{
  AppFunctionError, app_function, app_function_serializable,
};

/// Current track and whether audio is playing.
#[app_function_serializable]
#[derive(Clone)]
pub struct Playback {
  pub track_id: i32,
  pub title: String,
  pub playing: bool,
  pub volume: i32,
  pub position_ms: i64,
  pub duration_ms: i64,
}

#[cfg(target_os = "android")]
mod native {
  use super::*;
  use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JClass, JObject, JString, JValue},
  };
  use std::sync::OnceLock;
  static BRIDGE: OnceLock<(JavaVM, GlobalRef, GlobalRef)> = OnceLock::new();

  #[unsafe(no_mangle)]
  pub extern "system" fn Java_com_tauri_dev_music_MusicService_playbackChanged(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    json: JString<'_>,
  ) {
    use tauri::Emitter;
    // Notification buttons and audio-focus changes enter here too, without a UI command.
    if let Ok(text) = env.get_string(&json)
      && let Ok(state) = serde_json::from_str::<Playback>(&String::from(text))
    {
      if let Ok(app) = tauri_plugin_google_app_functions::app_handle::<tauri::Wry>() {
        let _ = app.emit("music-playback", state);
      }
    }
  }

  #[unsafe(no_mangle)]
  pub extern "system" fn Java_com_tauri_dev_music_MusicProvider_initialize(
    mut env: JNIEnv<'_>,
    _provider: JObject<'_>,
    context: JObject<'_>,
  ) {
    let result = (|| -> jni::errors::Result<()> {
      let class = env.find_class("com/tauri/dev/music/MusicService")?;
      let _ = BRIDGE.set((
        env.get_java_vm()?,
        env.new_global_ref(context)?,
        env.new_global_ref(class)?,
      ));
      Ok(())
    })();
    if let Err(error) = result {
      let _ = env.throw_new("java/lang/IllegalStateException", error.to_string());
    }
  }

  pub fn command(action: &str) -> Result<Playback, AppFunctionError> {
    let (vm, context, class) = BRIDGE
      .get()
      .ok_or_else(|| AppFunctionError::Unknown("music bridge not initialized".into()))?;
    let mut env = vm.attach_current_thread().map_err(failure)?;
    let action = env.new_string(action).map_err(failure)?;
    let class: &JClass<'_> = class.as_obj().into();
    let result = env.call_static_method(
      class,
      "command",
      "(Landroid/content/Context;Ljava/lang/String;)Ljava/lang/String;",
      &[
        JValue::Object(context.as_obj()),
        JValue::Object(action.as_ref()),
      ],
    );
    let value = match result {
      Ok(value) => value.l().map_err(failure)?,
      Err(error) => {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
        return Err(failure(error));
      }
    };
    let text: String = env
      .get_string(&JString::from(value))
      .map_err(failure)?
      .into();
    serde_json::from_str(&text).map_err(failure)
  }
  fn failure(error: impl std::fmt::Display) -> AppFunctionError {
    AppFunctionError::Unknown(error.to_string())
  }
}

pub fn command(action: &str) -> Result<Playback, AppFunctionError> {
  #[cfg(target_os = "android")]
  {
    native::command(action)
  }
  #[cfg(not(target_os = "android"))]
  {
    let _ = action;
    Err(AppFunctionError::NotSupported(
      "This minimal player example runs on Android".into(),
    ))
  }
}

/// Returns the current track without opening the app.
#[app_function]
fn playback_state() -> Result<Playback, AppFunctionError> {
  command("state")
}
/// Starts or resumes the selected track without opening the app.
#[app_function]
fn play_music() -> Result<Playback, AppFunctionError> {
  command("play")
}
/// Pauses music without opening the app.
#[app_function]
fn pause_music() -> Result<Playback, AppFunctionError> {
  command("pause")
}
/// Switches to the next of the two demo tracks and plays it without opening the app.
#[app_function]
fn next_track() -> Result<Playback, AppFunctionError> {
  command("next")
}
/// Switches to the previous demo track and plays it without opening the app.
#[app_function]
fn previous_track() -> Result<Playback, AppFunctionError> {
  command("previous")
}

#[tauri::command]
pub async fn music_command(action: String) -> Result<Playback, AppFunctionError> {
  match action.as_str() {
    "state" | "play" | "pause" | "next" | "previous" | "track:0" | "track:1" => {
      // Service lifecycle callbacks run on Android's main thread, so never wait there.
      tauri::async_runtime::spawn_blocking(move || command(&action))
        .await
        .map_err(|error| AppFunctionError::Unknown(error.to_string()))?
    }
    _ if action
      .strip_prefix("volume:")
      .and_then(|value| value.parse::<u8>().ok())
      .is_some_and(|value| value <= 100) =>
    {
      tauri::async_runtime::spawn_blocking(move || command(&action))
        .await
        .map_err(|error| AppFunctionError::Unknown(error.to_string()))?
    }
    _ => Err(AppFunctionError::InvalidArgument(
      "unknown player action".into(),
    )),
  }
}
