// A temporary page. It proves the app talks to the engine, and the real pages
// replace it. The design in the instructions comes later.

import { useEffect, useState } from "react";
import { getState, setPaused, type Snapshot } from "./lib/api";

function App() {
  const [state, setState] = useState<Snapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    try {
      setState(await getState());
      setError(null);
    } catch (problem) {
      setError(String(problem));
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function togglePause() {
    if (!state) return;
    await setPaused(!state.paused);
    refresh();
  }

  if (error) {
    return (
      <main style={{ fontFamily: "system-ui", padding: 24 }}>
        <h1>Leash</h1>
        <p style={{ color: "#DC2626" }}>The engine did not answer: {error}</p>
      </main>
    );
  }

  if (!state) {
    return (
      <main style={{ fontFamily: "system-ui", padding: 24 }}>
        <h1>Leash</h1>
        <p>Reading the services.</p>
      </main>
    );
  }

  return (
    <main style={{ fontFamily: "system-ui", padding: 24, lineHeight: 1.6 }}>
      <h1>Leash</h1>

      <p>
        {state.elevated
          ? "Running elevated, so stop actions are on."
          : "Leash needs administrator rights to stop services."}
      </p>
      <p>
        Watchdog: {state.watching ? "watching" : "stopped"}
        {state.paused ? ", paused" : ""}
      </p>
      <p>Caught today: {state.caught_today}</p>

      {state.config_error ? (
        <p style={{ color: "#B45309" }}>Config problem: {state.config_error}</p>
      ) : null}

      {state.unwatched.length > 0 ? (
        <p style={{ color: "#B45309" }}>
          Windows will not let Leash watch: {state.unwatched.join(", ")}
        </p>
      ) : null}

      <button onClick={togglePause} style={{ padding: "6px 12px" }}>
        {state.paused ? "Resume" : "Pause"}
      </button>

      <h2>On the leash</h2>
      {state.monitored.length === 0 ? (
        <p>Nothing yet.</p>
      ) : (
        <ul>
          {state.monitored.map((service) => (
            <li key={service.name}>
              {service.display_name} ({service.name}) — {service.policy}
            </li>
          ))}
        </ul>
      )}

      <h2>Last 7 days</h2>
      <ul>
        {state.weekly.slice(-7).map((day) => (
          <li key={day.date}>
            {day.date}: {day.count}
          </li>
        ))}
      </ul>
    </main>
  );
}

export default App;
