# Small Hours: a minimal Android music example

Two bundled tracks, play/pause, previous and next. The interface uses
[Radix Themes](https://www.radix-ui.com/themes/docs/overview/getting-started), inspired by its
[music app example](https://www.radix-ui.com/themes/example-music-app).

## Demo

[Watch the music demo (MP4)](demo.mp4). The same video is attached to the GitHub release.

## Run

Requires the Tauri Android toolchain and an Android 16+ device or emulator for AppFunctions.
From this directory:

```powershell
npm install
npm run tauri -- android dev
```

The tracks are original, synthesized 16-second loops: **Morning Loop** and **Night Drive**.
They are bundled with the app and loop continuously, with no network requests. Press a cover
or use the player controls. Closing the interface does not stop playback.

## Test from PowerShell with the interface closed

After installing the updated app:

```powershell
.\test-music.ps1
```

The script finds `adb`, force-stops the app, plays a track, switches next and previous, pauses,
and checks that no application Activity was opened. It also tests the system MediaSession's
Next, Previous, Pause and Play commands. It waits three seconds between switches
so you can hear the tracks. Playback is paused on completion, including after a failed check.
Use `-ListenSeconds 5` for longer pauses. No `test-functions` feature is needed.

| AppFunction | Action |
| --- | --- |
| `playMusic` | Play/resume the current track |
| `pauseMusic` | Pause playback |
| `nextTrack` | Play the other track |
| `previousTrack` | Play the previous track |
| `playbackState` | Read the title, track ID and playing state |

## How this small example works

- `src/App.tsx`: a compact Radix library, queue, volume control and bottom player. Local cover
  images are memoized separately from playback controls and the progress clock.
- `src/music.ts`: one initial state read, then `music-playback` events from Rust. There is no
  interval polling. Commands are queued, so quick Next/Previous presses are not discarded.
- `src-tauri/src/music.rs`: the same native player is called from UI commands and Rust
  AppFunctions. AppFunctions do not require an `AppHandle`.
- `src-tauri/android-music/MusicService.kt`: an Android `mediaPlayback` foreground service
  owns `MediaPlayer`. An initialization provider makes the bridge available even when
  the process starts without Tauri's Activity. There is no WebView audio element.
- `src-tauri/build.rs`: copies the example-only Kotlin and audio into the generated Android
  project and registers its provider, service and foreground-service permissions.

A system media notification shows the current cover, title, progress, Previous, Play/Pause
and Next buttons. The same MediaSession handles lock-screen/headset controls and seeking.
Playback handles audio focus and resumes the same track position after pause. The UI receives
changes from notification controls too, without polling or reloading the library artwork.
This small example has no streaming. Force-stopping the process stops the music; the next
AppFunction can start it again.

The root Android Back action moves the music screen's task to the background, allowing the
foreground playback service to continue. Reopening the app restores the existing screen.

Regenerate the audio with Python: `python src-tauri/android-music/generate_tracks.py`.
The two original loops are included under the repository's MIT license.

The earlier todo functions and `demo.ps1` remain available as a separate background-state
example. The visible interface is now the music player.
