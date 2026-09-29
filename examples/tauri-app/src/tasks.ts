import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Mirrors `Task` in src-tauri/src/tasks.rs. */
export interface Task {
  id: number;
  title: string;
  notes: string | null;
  done: boolean;
  /** Created by an agent through App Functions rather than in the app. */
  fromAgent: boolean;
}

export const loadTasks = () => invoke<Task[]>("tasks");
export const addTask = (title: string) => invoke<Task>("add_task", { title });
export const setTaskDone = (id: number, done: boolean) =>
  invoke<Task>("set_task_done", { id, done });
export const removeTask = (id: number) => invoke<void>("remove_task", { id });

/** Fires with the whole list on every change, including changes made by agents. */
export const onTasksChanged = (handler: (tasks: Task[]) => void): Promise<UnlistenFn> =>
  listen<Task[]>("tasks-changed", (event) => handler(event.payload));
