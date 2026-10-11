// Home. Section 11 puts five things in one centred column: the orbit and the
// counter, the quiet line, the held back cards, the latest catches, and the
// seven day bars.

import { useEffect, useMemo, useRef, useState } from "react";
import { Orbit, type Caught } from "../components/Orbit";
import { ServiceCard } from "../components/ServiceCard";
import { getLog } from "../lib/api";
import {
  clockTime,
  duration,
  entryIsOk,
  entryResult,
  entryText,
  lastSevenDays,
  weekdayLetter,
} from "../lib/format";
import type { Entry, Snapshot } from "../lib/types";

/** How many log lines Home reads. It covers the cards, the feed and the bars. */
const LOG_LIMIT = 200;

/** The most cards in one row, as section 11 asks. */
const MAX_CARDS = 6;

/** How wide a card is, with the gap, for counting what fits. */
const CARD_STEP = 150;

interface HomeProps {
  state: Snapshot;
  caught: Caught | null;
  /** Opens Services, for "view all". */
  onViewAll: () => void;
}

export function Home({ state, caught, onViewAll }: HomeProps) {
  const [log, setLog] = useState<Entry[]>([]);
  const [now, setNow] = useState(() => Date.now());
  const [fit, setFit] = useState(1);
  const cards = useRef<HTMLDivElement | null>(null);

  // The log holds what the cards and the feed need. It is read again whenever
  // the engine says something happened.
  useEffect(() => {
    let live = true;

    getLog(LOG_LIMIT)
      .then((entries) => {
        if (live) setLog(entries);
      })
      .catch(() => {
        if (live) setLog([]);
      });

    return () => {
      live = false;
    };
  }, [state.last_catch, state.caught_today]);

  // The quiet timer and the held timers move once a second.
  useEffect(() => {
    const tick = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(tick);
  }, []);

  // How many cards fit in the row, up to six.
  useEffect(() => {
    const measure = () => {
      const width = cards.current?.clientWidth ?? 0;
      setFit(Math.max(1, Math.min(MAX_CARDS, Math.floor((width + 10) / CARD_STEP))));
    };

    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, []);

  // When each service was last caught, newest first from the log.
  const lastCatches = useMemo(() => {
    const at = new Map<string, number>();
    for (const entry of log) {
      if (entry.action === "stop" && entry.result === "success") {
        const key = entry.service.toLowerCase();
        if (!at.has(key)) {
          at.set(key, Date.parse(entry.ts));
        }
      }
    }
    return at;
  }, [log]);

  // The cards: the service that was just caught first, then the latest catch.
  const onLeash = useMemo(() => {
    return state.monitored
      .map((service) => ({
        service,
        lastCatch: lastCatches.get(service.name.toLowerCase()) ?? null,
      }))
      .sort(
        (left, right) =>
          Number(caught?.name === right.service.name) -
            Number(caught?.name === left.service.name) ||
          (right.lastCatch ?? 0) - (left.lastCatch ?? 0),
      );
  }, [state.monitored, lastCatches, caught]);

  const quiet = state.paused
    ? "Watchdog paused"
    : state.last_catch
      ? `Quiet for ${duration(now - Date.parse(state.last_catch))}`
      : "Nothing caught yet";

  const feed = log.slice(0, 4);
  const days = lastSevenDays(state.weekly);
  const busiest = Math.max(...days.map((day) => day.count), 1);

  return (
    <div className="grid gap-2">
      <div className="pt-6 pb-2 text-center">
        <Orbit
          services={state.monitored}
          caught={caught}
          paused={state.paused}
          today={state.caught_today}
        />
        <p className="mt-[18px] font-mono text-[13px] text-muted">{quiet}</p>
      </div>

      <div>
        <Section title="Held back" count={onLeash.length} />

        <div ref={cards} className="grid grid-cols-[repeat(auto-fit,minmax(140px,1fr))] gap-2.5">
          {onLeash.slice(0, fit).map(({ service, lastCatch }) => (
            <ServiceCard
              key={service.name}
              service={service}
              lastCatch={lastCatch}
              now={now}
              justCaught={caught?.name === service.name}
              onOpen={onViewAll}
            />
          ))}
        </div>

        {onLeash.length === 0 ? (
          <p className="text-muted">Nothing on a leash yet. Pick a service in Services.</p>
        ) : null}

        {onLeash.length > fit ? (
          <button type="button" onClick={onViewAll} className="mt-3 rounded-pill border border-border px-3.5 py-[5px] text-xs font-medium text-muted hover:border-accent hover:text-text">
            View all {onLeash.length}
          </button>
        ) : null}

        <Section title="Caught" />
        <ul className="m-0 list-none p-0">
          {feed.map((entry, index) => (
            <li
              key={`${entry.ts}-${entry.service}-${index}`}
              className="flex items-baseline gap-3 border-t border-border py-[9px]"
            >
              <span className="font-mono text-xs text-muted">{clockTime(entry.ts)}</span>
              <span className="flex-1" title={entry.error ?? undefined}>
                {entryText(entry)}
              </span>
              <b
                className={`font-mono text-xs font-medium ${
                  entryIsOk(entry) ? "text-ok" : "text-warn"
                }`}
              >
                {entryResult(entry)}
              </b>
            </li>
          ))}
        </ul>

        <Section title="Last 7 days" />
        <div className="flex h-16 gap-2">
          {days.map((day, index) => (
            <div key={day.date} className="flex flex-1 items-end">
              <i
                className={`block w-full rounded-t-md rounded-b-sm transition-[height] duration-300 ${
                  index === 6 ? "bg-text/80" : "bg-border"
                }`}
                style={{ height: `${(day.count / busiest) * 100}%` }}
                title={`${day.count} caught`}
              />
            </div>
          ))}
        </div>
        <div className="mt-1.5 flex gap-2 font-mono text-[11px] text-muted">
          {days.map((day) => (
            <span key={day.date} className="flex-1 text-center">
              {weekdayLetter(day.date)}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}

/** A section heading, with the count when there is one. */
function Section({ title, count }: { title: string; count?: number }) {
  return (
    <h2 className="mt-[30px] mb-2.5 flex items-center justify-between text-sm font-medium text-muted">
      <span>
        {title}
        {count === undefined ? null : (
          <i className="ml-1.5 font-normal opacity-70">{count}</i>
        )}
      </span>
    </h2>
  );
}
