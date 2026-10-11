// Activity. Section 11: the full event list, with the time, the text, and the
// result. Every line the engine writes is here, including the ones Leash held
// back from acting on.

import { useEffect, useState } from "react";
import { getLog } from "../lib/api";
import { clockTime, entryResult, entryText, entryTone } from "../lib/format";
import type { Entry } from "../lib/types";
import { TONE_TEXT } from "../lib/ui";

/** How much of the log this page reads. */
const LOG_LIMIT = 500;

interface ActivityProps {
  /** Bumped by the app when something happened, to read the log again. */
  revision: number;
}

export function Activity({ revision }: ActivityProps) {
  const [entries, setEntries] = useState<Entry[] | null>(null);

  useEffect(() => {
    let live = true;

    getLog(LOG_LIMIT)
      .then((found) => {
        if (live) setEntries(found);
      })
      .catch(() => {
        if (live) setEntries([]);
      });

    return () => {
      live = false;
    };
  }, [revision]);

  return (
    <section>
      <h2 className="mt-0 mb-2.5 text-sm font-medium text-muted">Latest events</h2>

      {entries === null ? (
        <p className="text-muted">Reading the log.</p>
      ) : entries.length === 0 ? (
        <p className="text-muted">
          Nothing has happened yet. Leash writes a line here every time a service on a leash
          starts.
        </p>
      ) : (
        <ul className="m-0 list-none p-0">
          {entries.map((entry, index) => (
            <li
              key={`${entry.ts}-${entry.service}-${index}`}
              className="flex items-baseline gap-3 border-t border-border py-[9px]"
            >
              <span className="font-mono text-xs text-muted">{clockTime(entry.ts)}</span>
              <span className="flex-1" title={entry.error ?? undefined}>
                {entryText(entry)}
                {entry.error ? (
                  <span className="block font-mono text-xs text-muted">{entry.error}</span>
                ) : null}
              </span>
              <b className={`font-mono text-xs font-medium ${TONE_TEXT[entryTone(entry)]}`}>
                {entryResult(entry)}
              </b>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
