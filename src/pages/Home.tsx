// Home. Section 11 fills this with the orbit, the counter, the quiet timer, the
// held back cards, the latest catches, and the weekly bars. This is the plain
// first step: the numbers are real, the design comes next.

import type { Snapshot } from "../lib/types";

interface HomeProps {
  state: Snapshot;
}

export function Home({ state }: HomeProps) {
  const onLeash = state.monitored.length;

  return (
    <section>
      <p className="text-[13px] text-muted">
        {onLeash === 0
          ? "Nothing on a leash yet. Pick a service in Services."
          : `${onLeash} on a leash, ${state.caught_today} caught today.`}
      </p>
    </section>
  );
}
