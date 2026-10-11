// The shell: the icon rail, the top bar, the page switch, and one place that
// reads the state from the engine and keeps it fresh from the engine's events.

import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { Banner } from "./components/Banner";
import type { Caught } from "./components/Orbit";
import { Rail } from "./components/Rail";
import { ServiceSheet } from "./components/ServiceSheet";
import { TopBar } from "./components/TopBar";
import { getState, setPaused } from "./lib/api";
import { CAUGHT_HOLD_MS, REFRESH_EVENTS, SERVICE_CAUGHT, type Page } from "./lib/pages";
import type { Snapshot } from "./lib/types";
import { Activity } from "./pages/Activity";
import { Home } from "./pages/Home";
import { Services } from "./pages/Services";
import { Settings } from "./pages/Settings";

export default function App() {
  const [page, setPage] = useState<Page>("home");
  const [state, setState] = useState<Snapshot | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [caught, setCaught] = useState<Caught | null>(null);
  /** Bumped on every read, so the log page knows to read the log again. */
  const [revision, setRevision] = useState(0);
  /** The service whose sheet is open, or null. */
  const [service, setService] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setState(await getState());
      setProblem(null);
    } catch (error) {
      setProblem(String(error));
    } finally {
      setRevision((count) => count + 1);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // The engine tells the window when something happened, so the numbers on
  // screen do not wait for the next poll. A catch also flies out of the orbit
  // for a moment, which is what section 11 asks the orbit to show.
  useEffect(() => {
    let hold: number | undefined;
    const listening = [
      listen<{ service: string }>(SERVICE_CAUGHT, (event) => {
        setCaught({ name: event.payload.service, at: Date.now() });
        window.clearTimeout(hold);
        hold = window.setTimeout(() => setCaught(null), CAUGHT_HOLD_MS);
        void refresh();
      }),
      ...REFRESH_EVENTS.map((name) => listen(name, () => void refresh())),
    ];

    return () => {
      window.clearTimeout(hold);
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
    <>
      <div className="grid min-h-full grid-cols-[216px_minmax(0,1fr)] max-[719px]:grid-cols-1">
      <Rail
        page={page}
        onPick={setPage}
        elevated={state?.elevated ?? null}
        paused={state?.paused ?? null}
      />

      <main className="grid content-start gap-5 py-3.5 pr-6 pl-6 max-[719px]:gap-4 max-[719px]:px-3">
        <TopBar
          page={page}
          paused={state ? state.paused : null}
          onTogglePause={() => void togglePause()}
        />
        <Banner state={state} problem={problem} />

        {state ? (
          <Page
            page={page}
            state={state}
            caught={caught}
            revision={revision}
            onViewAll={() => setPage("services")}
            onChanged={() => void refresh()}
            onOpenService={setService}
          />
        ) : (
          <p className="text-[13px] text-muted">Reading the services.</p>
        )}
        </main>
      </div>

      <ServiceSheet
        name={service}
        onClose={() => setService(null)}
        onChanged={() => void refresh()}
      />
    </>
  );
}

/** The page that is showing. */
function Page({
  page,
  state,
  caught,
  revision,
  onViewAll,
  onChanged,
  onOpenService,
}: {
  page: Page;
  state: Snapshot;
  caught: Caught | null;
  revision: number;
  onViewAll: () => void;
  onChanged: () => void;
  onOpenService: (name: string) => void;
}) {
  switch (page) {
    case "home":
      return (
        <Home
          state={state}
          caught={caught}
          onViewAll={onViewAll}
          onOpenService={onOpenService}
        />
      );
    case "services":
      return <Services onChanged={onChanged} onOpenService={onOpenService} />;
    case "activity":
      return <Activity revision={revision} />;
    case "settings":
      return <Settings state={state} onChanged={onChanged} />;
  }
}

/** The entrance, so a page never appears statically. */
export const RISE = "animate-rise";
