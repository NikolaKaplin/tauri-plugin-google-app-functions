import { useEffect, useRef, useState, type FormEvent } from "react";
import Banner from "./Banner";
import { addTask, loadTasks, onTasksChanged, removeTask, setTaskDone, type Task } from "./tasks";

export default function App() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string | null>(null);
  // Tasks that just arrived from an agent get a short highlight.
  const [fresh, setFresh] = useState<Set<number>>(new Set());
  const known = useRef<Set<number> | null>(null);

  useEffect(() => {
    const apply = (next: Task[]) => {
      if (known.current) {
        const arrived = next.filter((t) => t.fromAgent && !known.current!.has(t.id)).map((t) => t.id);
        if (arrived.length) {
          setFresh((prev) => new Set([...prev, ...arrived]));
          setTimeout(() => {
            setFresh((prev) => new Set([...prev].filter((id) => !arrived.includes(id))));
          }, 2500);
        }
      }
      known.current = new Set(next.map((t) => t.id));
      setTasks(next);
    };
    loadTasks().then(apply).catch(showError);
    const unlisten = onTasksChanged(apply);
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  function showError(e: unknown) {
    const message = typeof e === "object" && e && "message" in e ? String(e.message) : String(e);
    setError(message);
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!draft.trim()) return;
    try {
      await addTask(draft);
      setDraft("");
      setError(null);
    } catch (e) {
      showError(e);
    }
  }

  const open = tasks.filter((t) => !t.done);
  const done = tasks.filter((t) => t.done);

  return (
    <main className="app">
      <Banner />

      <form className="composer" onSubmit={submit}>
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder="Add a task"
          aria-label="New task"
        />
        <button type="submit" disabled={!draft.trim()}>
          Add
        </button>
      </form>
      {error && <p className="error">{error}</p>}

      <section>
        <h2>
          To do <span className="count">{open.length}</span>
        </h2>
        {open.length === 0 ? (
          <p className="empty">Nothing to do. Ask Gemini to add something.</p>
        ) : (
          <TaskList tasks={open} fresh={fresh} onError={showError} />
        )}
      </section>

      {done.length > 0 && (
        <section>
          <h2>
            Done <span className="count">{done.length}</span>
          </h2>
          <TaskList tasks={done} fresh={fresh} onError={showError} />
        </section>
      )}
    </main>
  );
}

function TaskList(props: { tasks: Task[]; fresh: Set<number>; onError: (e: unknown) => void }) {
  return (
    <ul className="tasks">
      {props.tasks.map((task) => (
        <li key={task.id} className={`task${task.done ? " done" : ""}${props.fresh.has(task.id) ? " fresh" : ""}`}>
          <button
            className="check"
            aria-label={task.done ? "Mark as not done" : "Mark as done"}
            onClick={() => setTaskDone(task.id, !task.done).catch(props.onError)}
          />
          <div className="task-body">
            <span className="task-title">{task.title}</span>
            {task.notes && <span className="task-notes">{task.notes}</span>}
          </div>
          {task.fromAgent && (
            <span className="agent-chip" title="Added by an agent through App Functions">
              <img src="/gemini.svg" alt="" />
              Gemini
            </span>
          )}
          <span className="task-id">#{task.id}</span>
          <button
            className="remove"
            aria-label="Delete task"
            onClick={() => removeTask(task.id).catch(props.onError)}
          >
            ×
          </button>
        </li>
      ))}
    </ul>
  );
}
