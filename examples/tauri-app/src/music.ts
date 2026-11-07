import { useSyncExternalStore } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface Playback {
  trackId: number;
  title: string;
  playing: boolean;
  volume: number;
  positionMs: number;
  durationMs: number;
}
export const tracks = [
  { id: 0, title: "Morning Loop", artist: "Small Hours", album: "Morning sessions", mood: "Warm, mellow instrumentals", cover: "/covers/morning.svg" },
  { id: 1, title: "Night Drive", artist: "Small Hours", album: "After dark", mood: "For taking the long way home", cover: "/covers/night.svg" },
] as const;

const initial: Playback = { trackId: 0, title: tracks[0].title, playing: false, volume: 70, positionMs: 0, durationMs: 16000 };
let snapshot = { playback: initial, busy: false, error: "", receivedAt: Date.now() };
const subscribers = new Set<() => void>();
let revision = 0;
let pending = 0;
let commands = Promise.resolve();
const subscribe = (listener: () => void) => { subscribers.add(listener); return () => { subscribers.delete(listener); }; };
const getSnapshot = () => snapshot;
function publish(update: Partial<typeof snapshot>) {
  snapshot = { ...snapshot, ...update };
  subscribers.forEach((listener) => listener());
}
function receive(playback: Playback) {
  revision++;
  const previous = snapshot.playback;
  if ((Object.keys(playback) as (keyof Playback)[]).some((key) => playback[key] !== previous[key])) {
    publish({ playback, receivedAt: Date.now() });
  }
}
export const usePlayer = () => useSyncExternalStore(subscribe, getSnapshot, getSnapshot);

// Serialize commands rather than dropping quick Next/Previous presses. Cover components never
// subscribe to playback, busy state, or the progress clock.
export function send(action: string): void {
  revision++;
  if (++pending === 1) publish({ busy: true, error: "" });
  commands = commands.then(async () => {
    try { receive(await invoke<Playback>("music_command", { action })); }
    catch (error) {
      publish({ error: typeof error === "object" && error && "message" in error ? String(error.message) : String(error) });
    } finally { if (--pending === 0) publish({ busy: false }); }
  });
}

async function refresh(isAlive: () => boolean) {
  const before = revision;
  try {
    const playback = await invoke<Playback>("music_command", { action: "state" });
    if (isAlive() && before === revision && pending === 0) receive(playback);
  } catch { /* A plain browser can preview the library; playback is native to Android. */ }
}

// One initial read, then native events (including notification buttons). No interval polling.
export function connectPlayer(): () => void {
  let alive = true;
  let unlisten: (() => void) | undefined;
  const sync = () => { if (!document.hidden) void refresh(() => alive); };
  void listen<Playback>("music-playback", (event) => { if (alive) receive(event.payload); })
    .then((stop) => { if (!alive) stop(); else { unlisten = stop; sync(); } })
    .catch(() => { if (alive) sync(); });
  window.addEventListener("focus", sync);
  document.addEventListener("visibilitychange", sync);
  return () => {
    alive = false;
    unlisten?.();
    window.removeEventListener("focus", sync);
    document.removeEventListener("visibilitychange", sync);
  };
}
