//! JNI entry point called by `AppFunctionsBridge.kt` for every app function execution.

use std::panic::{AssertUnwindSafe, catch_unwind};

use jni::{
  JNIEnv,
  objects::{JClass, JString},
  sys::jstring,
};
use serde_json::{Value, json};

use crate::registry::{self, AppFunctionError};

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_plugin_google_1app_1functions_AppFunctionsBridge_nativeInvoke<
  'local,
>(
  mut env: JNIEnv<'local>,
  _class: JClass<'local>,
  function: JString<'local>,
  args: JString<'local>,
) -> jstring {
  let result = catch_unwind(AssertUnwindSafe(|| invoke(&mut env, &function, &args)))
    .unwrap_or_else(|panic| {
      let message = panic
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "app function panicked".to_owned());
      Err(AppFunctionError::Unknown(message))
    });

  let response = match result {
    Ok(value) => json!({ "ok": value }),
    Err(error) => json!({ "error": error }),
  };

  env
    .new_string(response.to_string())
    .map(|s| s.into_raw())
    .unwrap_or(std::ptr::null_mut())
}

fn invoke(
  env: &mut JNIEnv<'_>,
  function: &JString<'_>,
  args: &JString<'_>,
) -> Result<Value, AppFunctionError> {
  let jni_error = |e: jni::errors::Error| AppFunctionError::Unknown(e.to_string());
  let function: String = env.get_string(function).map_err(jni_error)?.into();
  let args: String = env.get_string(args).map_err(jni_error)?.into();
  let args: Value = serde_json::from_str(&args).map_err(registry::invalid_arguments)?;
  tauri::async_runtime::block_on(registry::invoke(&function, args))
}
