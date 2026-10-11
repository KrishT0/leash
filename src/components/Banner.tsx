// The warnings that sit above the page. Section 6 asks for the rights banner,
// and a config file that could not be read is worth showing too, because Leash
// refuses to write over it.

import type { Snapshot } from "../lib/types";

interface BannerProps {
  state: Snapshot | null;
  /** A command that failed, in the words the engine used. */
  problem: string | null;
}

export function Banner({ state, problem }: BannerProps) {
  const warnings: string[] = [];

  if (state && !state.elevated) {
    warnings.push("Leash needs administrator rights to stop services.");
  }
  if (state?.config_error) {
    warnings.push(`The config file could not be read: ${state.config_error}`);
  }
  if (problem) {
    warnings.push(problem);
  }
  if (warnings.length === 0) {
    return null;
  }

  return (
    <div className="grid gap-2">
      {warnings.map((warning) => (
        <p
          key={warning}
          className="m-0 rounded-island border border-warn/25 bg-warn/10 px-4 py-3 text-[13px] text-warn backdrop-blur-xl"
        >
          {warning}
        </p>
      ))}
    </div>
  );
}
