# Tauri Todo: a demo of tauri-plugin-google-app-functions

A todo list built with Tauri, React and Rust. Its Rust functions are exposed as Android
[AppFunctions](https://developer.android.com/ai/appfunctions), so Gemini and other on-device
agents can add, read, finish and delete tasks. When an agent changes the list, the task shows up
in the app right away, with a Gemini badge.

<p align="center">
  <img src="../../.github/demo.gif" alt="Demo: app functions called over adb, the task shows up in the app" width="100%">
</p>

## Demo

The recording above runs [demo.ps1](demo.ps1): it creates a task, reads it back, lists the open
tasks, completes it and asks for a missing one. The same calls one by one:

### Create a task with adb

```sh
adb shell "cmd app_function execute-app-function --package com.tauri.dev \
  --function 'com.plugin.google_app_functions.TauriAppFunctionService#createTask' \
  --parameters '{\"title\": \"Buy milk\", \"notes\": \"2 liters\"}'"
```

### View a task with adb

```sh
adb shell "cmd app_function execute-app-function --package com.tauri.dev \
  --function 'com.plugin.google_app_functions.TauriAppFunctionService#getTask' \
  --parameters '{\"id\": 1}'"
```

### Ask Gemini

> _"Add buy milk to my list in Tauri Todo"_

Whether Gemini calls the app depends on Google's rollout; see
[Compatibility](../../README.md#compatibility).

## App functions

| Function | What it does |
| --- | --- |
| `createTask(title, notes?)` | Adds a task and returns it with its ID. |
| `listTasks(includeDone?)` | Lists the tasks, oldest first; `includeDone = false` leaves out finished ones. |
| `getTask(id)` | Returns one task, or `ElementNotFound`. |
| `completeTask(id)` | Marks a task as done. |
| `deleteTask(id)` | Deletes a task. |

All of them live in [src-tauri/src/lib.rs](src-tauri/src/lib.rs); the list itself is in
[src-tauri/src/tasks.rs](src-tauri/src/tasks.rs) and is saved to the app's data directory. The
functions take a `tauri::AppHandle`, so they need the app to be open: while it is closed, they
fail with _"the app is not running"_.

## Run it

Requires an Android 16+ device or emulator (see [Compatibility](../../README.md#compatibility)).

```sh
npm install
npm run tauri android dev
```

Then list what the system indexed:

```sh
adb shell cmd app_function list-app-functions | grep -A2 com.tauri.dev
```

## Tests

[test-app-functions.sh](test-app-functions.sh) calls every function over `adb`, with the app
closed and open, including a set of test functions that cover every supported type and error
code. The test functions are only built with a feature flag, so agents don't see them in the
demo:

```sh
npm run tauri android dev -- --features test-functions
./test-app-functions.sh
```
