use std::{any::Any, future::Future, pin::Pin, sync::OnceLock};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Runtime};

/// Error returned by an app function.
///
/// On Android each variant is rethrown as the matching `androidx.appfunctions` exception,
/// so the calling agent can tell *why* the call failed.
#[derive(Debug, Clone, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum AppFunctionError {
  /// `AppFunctionInvalidArgumentException`
  #[error("{0}")]
  InvalidArgument(String),
  /// `AppFunctionElementNotFoundException`
  #[error("{0}")]
  ElementNotFound(String),
  /// `AppFunctionElementAlreadyExistsException`
  #[error("{0}")]
  ElementAlreadyExists(String),
  /// `AppFunctionPermissionRequiredException`
  #[error("{0}")]
  PermissionRequired(String),
  /// `AppFunctionNotSupportedException`
  #[error("{0}")]
  NotSupported(String),
  /// `AppFunctionDisabledException`
  #[error("{0}")]
  Disabled(String),
  /// `AppFunctionLimitExceededException`
  #[error("{0}")]
  LimitExceeded(String),
  /// `AppFunctionCancelledException`
  #[error("{0}")]
  Cancelled(String),
  /// `AppFunctionFunctionNotFoundException`
  #[error("{0}")]
  FunctionNotFound(String),
  /// `AppFunctionAppUnknownException`
  #[error("{0}")]
  Unknown(String),
}

impl From<String> for AppFunctionError {
  fn from(message: String) -> Self {
    Self::Unknown(message)
  }
}

impl From<&str> for AppFunctionError {
  fn from(message: &str) -> Self {
    Self::Unknown(message.to_owned())
  }
}

impl From<tauri::Error> for AppFunctionError {
  fn from(error: tauri::Error) -> Self {
    Self::Unknown(error.to_string())
  }
}

impl From<crate::Error> for AppFunctionError {
  fn from(error: crate::Error) -> Self {
    Self::Unknown(error.to_string())
  }
}

#[doc(hidden)]
pub type BoxFuture = Pin<Box<dyn Future<Output = Result<Value, AppFunctionError>> + Send>>;

#[doc(hidden)]
pub struct AppFunctionEntry {
  pub name: &'static str,
  pub handler: fn(Value) -> BoxFuture,
}

inventory::collect!(AppFunctionEntry);

/// Names of all functions registered with `#[app_function]`.
pub fn registered_functions() -> impl Iterator<Item = &'static str> {
  inventory::iter::<AppFunctionEntry>.into_iter().map(|entry| entry.name)
}

/// Calls a registered app function by its Rust name with JSON arguments keyed by the
/// camelCase parameter names, exactly like Android does. Useful for testing on desktop.
pub async fn invoke(name: &str, args: Value) -> Result<Value, AppFunctionError> {
  let entry = inventory::iter::<AppFunctionEntry>
    .into_iter()
    .find(|entry| entry.name == name)
    .ok_or_else(|| AppFunctionError::FunctionNotFound(format!("no app function named `{name}`")))?;
  (entry.handler)(args).await
}

static APP_HANDLE: OnceLock<Box<dyn Any + Send + Sync>> = OnceLock::new();

pub(crate) fn set_app_handle<R: Runtime>(app: AppHandle<R>) {
  let _ = APP_HANDLE.set(Box::new(app));
}

/// Returns the running app's handle.
///
/// Android can start the app function service without opening the app UI, in which case the
/// Tauri app was never built and this fails with [`AppFunctionError::Unknown`].
pub fn app_handle<R: Runtime>() -> Result<AppHandle<R>, AppFunctionError> {
  APP_HANDLE
    .get()
    .and_then(|handle| handle.downcast_ref::<AppHandle<R>>())
    .cloned()
    .ok_or_else(|| {
      AppFunctionError::Unknown(
        "the app is not running; open it and retry".to_owned(),
      )
    })
}

#[doc(hidden)]
pub fn serialize_output<T: Serialize>(value: T) -> Result<Value, AppFunctionError> {
  serde_json::to_value(value)
    .map_err(|e| AppFunctionError::Unknown(format!("failed to serialize the result: {e}")))
}

#[doc(hidden)]
pub fn serialize_result<T: Serialize, E: Into<AppFunctionError>>(
  result: Result<T, E>,
) -> Result<Value, AppFunctionError> {
  serialize_output(result.map_err(Into::into)?)
}

#[doc(hidden)]
pub fn invalid_arguments(error: serde_json::Error) -> AppFunctionError {
  AppFunctionError::InvalidArgument(format!("invalid arguments: {error}"))
}
