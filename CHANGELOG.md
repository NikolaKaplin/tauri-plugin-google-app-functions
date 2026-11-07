# Changelog

## 0.2.0 — 2026-11-07

### Changed

- Upgrade Jetpack AppFunctions from `1.0.0-alpha12` to `1.0.0-rc01`.
- Generate the `AppFunctionAppMetadata` XML `instructions` attribute used by the RC.
  The Rust `Builder::app_description()` API is unchanged.

### Added

- Android-only `android_app_data_dir()`: access the private application data directory during
  AppFunction execution, including a cold service start without an Activity or WebView.
- A shared, lazily initialized task-store example that persists background changes and reuses
  the same state when the Tauri interface opens.
- Small Hours, a minimal two-track music example with a Radix Themes interface, native Android
  playback, a media notification and lock-screen transport controls. Player changes use events
  instead of polling; static cover art is isolated from playback and progress updates.
- PowerShell checks for background task persistence, track switching and system media controls.

### Upgrading from 0.1

Upgrade both `tauri-plugin-google-app-functions` and `tauri-plugin-google-app-functions-build`
to `0.2`, then rebuild Android through the Tauri CLI so metadata and native code update together.
If you supply your own `AppFunctionAppMetadata` XML, rename its `appfn:description` attribute
to `appfn:instructions`.

An `AppHandle` still requires an initialized Tauri application. Background handlers should
initialize their own shared Rust state; `android_app_data_dir()` is available during Android
AppFunction execution and returns an error before the service supplies its directory.
