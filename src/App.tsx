// The shell: the icon rail, the top bar, the page switch, and one place that
// reads the state from the engine and keeps it fresh from the engine's events.

import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { Banner } from "./components/Banner";
import { Rail } from "./components/Rail";
import { TopBar } from "./components/TopBar";
import { getState, setPaused } from "./lib/api";
import { ENGINE_EVENTS, type Page } from "./lib/pages";
import type { Snapshot } from "./lib/types";
import { Activity } from "./pages/Activity";
import { Home } from "./pages/Home";
import { Services } from "./pages/Services";
import { Settings } from "./pages/Settings";

export default function App() {
  const [page, setPage] = useState<Page>("home");
  const [state, setState] = useState<Snapshot | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setState(await getState());
      setProblem(null);
    } catch (error) {
      setProblem(String(error));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // The engine tells the window when something happened, so the numbers on
  // screen do not wait for the next poll.
  useEffect(() => {
    const listening = ENGINE_EVENTS.map((name) => listen(name, () => void refresh()));
    return () => {
      for (const pending of listening) {
        void pending.then((stop) => stop());
      }
    };
  }, [refresh]);

  async function togglePause() {
    if (!state) return;

    try {
      await setPaused(!state.paused);
      await refresh();
    } catch (error) {
      setProblem(String(error));
    }
  }

  return (
    <div className="grid min-h-full grid-cols-[68px_minmax(0,1fr)] max-[719px]:grid-cols-1">
      <Rail page={page} onPick={setPage} />

      <main className="mx-auto w-full max-w-[1080px] px-8 pt-6 pb-12 max-[719px]:px-3.5 max-[719px]:pt-4 max-[719px]:pb-10">
        <TopBar
          page={page}
          paused={state ? state.paused : null}
          onTogglePause={() => void togglePause()}
        />
        <Banner state={state} problem={problem} />

        {state ? (
          <Page page={page} state={state} />
        ) : (
          <p className="text-[13px] text-muted">Reading the services.</p>
        )}
      </main>
    </div>
  );
}

/** The page that is showing. */
function Page({ page, state }: { page: Page; state: Snapshot }) {
  switch (page) {
    case "home":
      return <Home state={state} />;
    case "services":
      return <Services state={state} />;
    case "activity":
      return <Activity />;
    case "settings":
      return <Settings state={state} />;
  }
}
