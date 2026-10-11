// Activity. Section 11 gives this the full event list with the colours for each
// result. This is the plain first step, and it reads the real log.

import { useEffect, useState } from "react";
import { getLog } from "../lib/api";
import type { Entry } from "../lib/types";

export function Activity() {
  const [entries, setEntries] = useState<Entry[] | null>(null);

  useEffect(() => {
    getLog(200)
      .then(setEntries)
      .catch(() => setEntries([]));
  }, []);

  if (!entries) {
    return <p className="text-[13px] text-muted">Reading the log.</p>;
  }

  return (
    <section>
      <p className="text-[13px] text-muted">
        {entries.length === 0
          ? "Nothing has happened yet."
          : `${entries.length} events in the log.`}
      </p>
    </section>
  );
}
