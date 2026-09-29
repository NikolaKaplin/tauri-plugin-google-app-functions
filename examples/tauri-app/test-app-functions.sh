#!/usr/bin/env bash
# Calls every app function of the example on a connected device/emulator (Android 16+).
# Build the app with the test functions first:
#   npm run tauri android dev -- --features test-functions
# Usage: ./test-app-functions.sh [path/to/app.apk]
set -u

PKG=com.tauri.dev
PREFIX=com.plugin.google_app_functions.TauriAppFunctionService
ADB=${ADB:-adb}

if [ $# -ge 1 ]; then
  "$ADB" install -r "$1" || exit 1
fi

call() {
  local fn=$1 params=$2 expect=$3
  echo
  echo "=== $fn  (expected: $expect)"
  echo "    params: $params"
  "$ADB" shell "cmd app_function execute-app-function --package $PKG --function '$PREFIX#$fn' --parameters '$params'"
}

echo "=== Registered functions of $PKG"
"$ADB" shell cmd app_function list-app-functions | grep -A2 "$PKG" || \
  echo "!!! $PKG not found: the system did not index the functions"

"$ADB" shell am force-stop "$PKG"
echo
echo "##### App closed: the service must start without the UI"
call testCountWords '{"text": "hello app functions"}' '3'
call testEchoPrimitives '{"flag": true, "small": -7, "big": 9007199254740993, "ratio": 1.5, "precise": 3.141592653589793, "text": "Привет $ \"q\""}' 'same values'
call testSum '{"values": [1, 2, 3, 2147483647]}' '2147483653'
call testScale '{"values": [1.5, -2], "factor": 2}' '[3.0, -4.0]'
call testJoin '{"words": ["a", "b", "c"]}' '"a b c" (separator omitted)'
call testJoin '{"words": ["a", "b"], "separator": "-"}' '"a-b"'
call testContact '{"name": "Ann", "tags": ["friend"], "home": {"city": "Paris", "street": "Rue 1"}, "otherAddresses": [{"city": "Rome"}]}' 'nested contact, luckyNumbers [3,1,2]'
call testContact '{"name": "Bob", "tags": [], "otherAddresses": []}' 'home null, empty lists'
call testNoop '{}' 'Unit, no error'
for kind in invalidArgument elementNotFound elementAlreadyExists permissionRequired notSupported disabled limitExceeded cancelled unknown; do
  call testFail "{\"kind\": \"$kind\"}" "error matching $kind"
done
call testPanic '{}' 'AppUnknown error "test panic from Rust", app keeps running'
call testAppName '{}' 'error: the app is not running'
call createTask '{"title": "x"}' 'error: the app is not running'

echo
echo "##### Opening the app"
"$ADB" shell monkey -p "$PKG" -c android.intent.category.LAUNCHER 1 > /dev/null
sleep 5
call testAppName '{}' '"Tauri Todo"'
call createTask '{"title": "Buy milk", "notes": "2 liters"}' 'new task, fromAgent true; shows up in the app'
call createTask '{"title": "  "}' 'InvalidArgument: title must not be empty'
call listTasks '{}' 'all tasks'
call listTasks '{"includeDone": false}' 'open tasks only'
call getTask '{"id": 1}' 'task 1'
call getTask '{"id": 999999}' 'ElementNotFound: no task with id 999999'
call completeTask '{"id": 1}' 'task 1 with done true'
call deleteTask '{"id": 999999}' 'ElementNotFound: no task with id 999999'
