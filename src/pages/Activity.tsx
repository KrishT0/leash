// Activity. Section 11: the full event list, with the time, the text, and the
// result. Every line the engine writes is here, including the ones Leash held
// back from acting on.

import { useEffect, useState } from "react";
import { Tag } from "../components/Tag";
import { getLog } from "../lib/api";
import { clockTime, entryResult, entryText, entryTone } from "../lib/format";
import type { Entry } from "../lib/types";
import { EYEBROW } from "../lib/ui";

/** How much of the log this page reads. */
const LOG_LIMIT = 500;

interface ActivityProps {
  /** Bumped by the app when something happened, to read the log again. */
  revision: number;
}

export function Activity({ revision }: ActivityProps) {
  const [entries, setEntries] = useState<Entry[] | null>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: re-read when the app reports a change.
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
    <section className="grid content-start gap-4">
      <p className={`m-0 ${EYEBROW}`}>Latest events</p>

      {entries === null ? (
        <p className="m-0 text-[12.5px] text-muted">Reading the log.</p>
      ) : entries.length === 0 ? (
        <p className="m-0 text-[12.5px] text-muted">
          Nothing has happened yet. Leash writes a line here every time a service on a leash
          starts.
        </p>
      ) : (
        <ul className="m-0 list-none p-0">
          {entries.map((entry, index) => (
            <li
              /* biome-ignore lint/suspicious/noArrayIndexKey: two log lines can be identical to the second. */
              key={`${entry.ts}-${entry.service}-${index}`}
              className="flex items-center gap-3 border-t border-hair py-3 first:border-t-0 first:pt-0"
            >
              <span className="font-mono text-[10.5px] text-muted">{clockTime(entry.ts)}</span>
              <span className="flex-1 text-[12.5px]" title={entry.error ?? undefined}>
                {entryText(entry)}
                {entry.error ? (
                  <span className="mt-1 block font-mono text-[10.5px] text-muted">
                    {entry.error}
                  </span>
                ) : null}
              </span>
              <Tag tone={entryTone(entry)}>{entryResult(entry)}</Tag>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
