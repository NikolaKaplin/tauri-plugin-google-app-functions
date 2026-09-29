//! App functions that exercise every supported type mapping and error path of the plugin.
//! Call them with `adb shell cmd app_function execute-app-function` (see the example README).

use tauri::AppHandle;
use tauri_plugin_google_app_functions::{
    AppFunctionError, app_function, app_function_serializable,
};

/// Every primitive parameter echoed back.
#[app_function_serializable]
pub struct PrimitivesEcho {
    /// Echo of `flag`.
    pub flag: bool,
    /// Echo of `small`.
    pub small: i32,
    /// Echo of `big`.
    pub big: i64,
    /// Echo of `ratio`.
    pub ratio: f32,
    /// Echo of `precise`.
    pub precise: f64,
    /// Echo of `text`.
    pub text: String,
}

/// A postal address.
#[app_function_serializable]
#[derive(Clone)]
pub struct Address {
    /// City name.
    pub city: String,
    /// Street and house number, if known.
    pub street: Option<String>,
}

/// A contact with nested objects and lists.
#[app_function_serializable]
pub struct Contact {
    /// Full name.
    pub name: String,
    /// Free-form labels.
    pub tags: Vec<String>,
    /// Main address, if any.
    pub home: Option<Address>,
    /// All known addresses.
    pub addresses: Vec<Address>,
    /// Lucky numbers.
    pub lucky_numbers: Vec<i64>,
}

/// Returns all primitive arguments unchanged.
///
/// # Arguments
/// * `flag` - Any boolean.
/// * `small` - Any 32-bit integer.
/// * `big` - Any 64-bit integer.
/// * `ratio` - Any 32-bit float.
/// * `precise` - Any 64-bit float.
/// * `text` - Any text.
#[app_function]
fn test_echo_primitives(
    flag: bool,
    small: i32,
    big: i64,
    ratio: f32,
    precise: f64,
    text: String,
) -> PrimitivesEcho {
    PrimitivesEcho {
        flag,
        small,
        big,
        ratio,
        precise,
        text,
    }
}

/// Sums a list of integers.
///
/// # Arguments
/// * `values` - Integers to add up.
#[app_function]
fn test_sum(values: Vec<i32>) -> i64 {
    values.iter().map(|&v| v as i64).sum()
}

/// Multiplies each number by a factor.
///
/// # Arguments
/// * `values` - Numbers to scale.
/// * `factor` - Multiplier.
#[app_function]
fn test_scale(values: Vec<f64>, factor: f64) -> Vec<f64> {
    values.into_iter().map(|v| v * factor).collect()
}

/// Joins words with a separator.
///
/// # Arguments
/// * `words` - Words to join.
/// * `separator` - Separator between words; a single space when omitted.
#[app_function]
async fn test_join(words: Vec<String>, separator: Option<String>) -> String {
    words.join(separator.as_deref().unwrap_or(" "))
}

/// Builds a contact from nested input and echoes it back with derived fields.
///
/// # Arguments
/// * `name` - Contact name.
/// * `tags` - Labels for the contact.
/// * `home` - Main address; optional.
/// * `other_addresses` - Additional addresses.
#[app_function]
fn test_contact(
    name: String,
    tags: Vec<String>,
    home: Option<Address>,
    other_addresses: Vec<Address>,
) -> Contact {
    let mut addresses: Vec<Address> = home.iter().cloned().collect();
    addresses.extend(other_addresses);
    Contact {
        lucky_numbers: vec![name.len() as i64, tags.len() as i64, addresses.len() as i64],
        name,
        tags,
        home,
        addresses,
    }
}

/// Always fails with the requested error kind.
///
/// # Arguments
/// * `kind` - One of: invalidArgument, elementNotFound, elementAlreadyExists,
///   permissionRequired, notSupported, disabled, limitExceeded, cancelled, unknown.
#[app_function]
fn test_fail(kind: String) -> Result<(), AppFunctionError> {
    let message = format!("requested failure: {kind}");
    Err(match kind.as_str() {
        "invalidArgument" => AppFunctionError::InvalidArgument(message),
        "elementNotFound" => AppFunctionError::ElementNotFound(message),
        "elementAlreadyExists" => AppFunctionError::ElementAlreadyExists(message),
        "permissionRequired" => AppFunctionError::PermissionRequired(message),
        "notSupported" => AppFunctionError::NotSupported(message),
        "disabled" => AppFunctionError::Disabled(message),
        "limitExceeded" => AppFunctionError::LimitExceeded(message),
        "cancelled" => AppFunctionError::Cancelled(message),
        _ => AppFunctionError::Unknown(message),
    })
}

/// Does nothing and returns no value.
#[app_function]
fn test_noop() {}

/// Panics inside Rust; the call must fail with an error instead of crashing the app.
#[app_function]
fn test_panic() -> String {
    panic!("test panic from Rust")
}

/// Returns the app name. Fails while the app has not been opened, because it needs the
/// running Tauri app.
#[app_function]
fn test_app_name(app: AppHandle) -> String {
    app.package_info().name.clone()
}
