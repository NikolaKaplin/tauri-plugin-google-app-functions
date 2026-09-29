#!/usr/bin/env bash
# Calls every test app function of the example on a connected device/emulator (Android 16+).
# Usage: ./test-app-functions.sh [path/to/app.apk]
set -u

PKG=com.tauri.dev
PREFIX=com.plugin.google_app_functions.generated.TauriAppFunctionServiceBase
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
call countWords '{"text": "hello app functions"}' '3'
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
call createNote '{"title": "x"}' 'error: the app is not running'

echo
echo "##### Opening the app"
"$ADB" shell monkey -p "$PKG" -c android.intent.category.LAUNCHER 1 > /dev/null
sleep 5
call testAppName '{}' '"tauri-app"'
call createNote '{"title": "Groceries", "content": "milk"}' 'note id 1'
call createNote '{"title": "Call mom"}' 'note id 2, content null'
call createNote '{"title": "  "}' 'InvalidArgument: title must not be empty'
call listNotes '{}' 'both notes'
