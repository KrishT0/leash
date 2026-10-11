// Home, as the design reference composes it: one column. The orbit with the
// counter inside it, the quiet line under it, then the services held back, the
// latest catches, and the seven day bars. No cards here, so the orbit keeps the
// whole width above the lists.

import { useEffect, useMemo, useRef, useState } from "react";
import { Orbit, type Caught } from "../components/Orbit";
import { ServiceCard } from "../components/ServiceCard";
import { Tag } from "../components/Tag";
import { getLog } from "../lib/api";
import {
  clockTime,
  duration,
  entryResult,
  entryText,
  entryTone,
  lastSevenDays,
  weekdayLetter,
} from "../lib/format";
import type { Entry, Snapshot } from "../lib/types";
import { EYEBROW, GHOST } from "../lib/ui";

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
  /** Opens the service sheet, when a dot on the orbit is clicked. */
  onOpenService: (name: string) => void;
}

export function Home({ state, caught, onViewAll, onOpenService }: HomeProps) {
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
    <div className="grid content-start">
      <div className="pt-5 pb-2 text-center">
        <Orbit
          services={state.monitored}
          caught={caught}
          paused={state.paused}
          today={state.caught_today}
          onPick={onOpenService}
        />
        <p className="mt-4 font-mono text-[12.5px] text-muted">{quiet}</p>
      </div>

      <Section title="Held back">
        {onLeash.length > 0 ? <Tag tone="dim">{onLeash.length} on a leash</Tag> : null}
      </Section>

      {onLeash.length === 0 ? (
        <p className="m-0 text-[13px] text-muted">
          Nothing on a leash yet. Pick a service in Services.
        </p>
      ) : (
        <div
          ref={cards}
          className="grid grid-cols-[repeat(auto-fit,minmax(140px,1fr))] gap-2.5"
        >
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
      )}

      {onLeash.length > fit ? (
        <button type="button" onClick={onViewAll} className={`${GHOST} mt-3 justify-self-start`}>
          View all {onLeash.length}
        </button>
      ) : null}

      <Section title="Caught" />

      <ul className="m-0 list-none p-0">
        {feed.map((entry, index) => (
          <li
            key={`${entry.ts}-${entry.service}-${index}`}
            className="flex items-center gap-3 border-t border-hair py-3 first:border-t-0 first:pt-0"
          >
            <span className="font-mono text-[11px] text-muted">{clockTime(entry.ts)}</span>
            <span className="flex-1 text-[13px]" title={entry.error ?? undefined}>
              {entryText(entry)}
            </span>
            <Tag tone={entryTone(entry)}>{entryResult(entry)}</Tag>
          </li>
        ))}
        {feed.length === 0 ? (
          <li className="text-[13px] text-muted">Nothing has happened yet.</li>
        ) : null}
      </ul>

      <Section title="Last 7 days" />

      <div className="flex h-16 gap-2">
        {days.map((day, index) => (
          <i
            key={day.date}
            className={`block w-full self-end rounded-t-[10px] rounded-b-[3px] transition-[height] duration-700 ease-spring ${
              index === 6
                ? "bg-accent shadow-[0_0_24px_color-mix(in_srgb,var(--color-accent)_45%,transparent)]"
                : "bg-border"
            }`}
            style={{ height: `${Math.max(6, (day.count / busiest) * 100)}%` }}
            title={`${day.count} caught`}
          />
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
  );
}

/** A section heading, with anything that belongs on the same line. */
function Section({ title, children }: { title: string; children?: React.ReactNode }) {
  return (
    <h2 className={`mt-7 mb-2.5 flex items-center justify-between gap-3 ${EYEBROW}`}>
      <span>{title}</span>
      {children}
    </h2>
  );
}
