// The warnings that sit above the page. Section 6 asks for the rights banner,
// with the way out of it beside the sentence. A config file that could not be
// read is worth showing too, because Leash refuses to write over it.

import type { Snapshot } from "../lib/types";

interface BannerProps {
  state: Snapshot | null;
  /** A command that failed, in the words the engine used. */
  problem: string | null;
  /** Section 6: start Leash again through the scheduled task, with rights. */
  onRestartAsAdmin: () => void;
}

export function Banner({ state, problem, onRestartAsAdmin }: BannerProps) {
  const needsRights = state !== null && !state.elevated;

  const warnings: string[] = [];
  if (state?.config_error) {
    warnings.push(`The config file could not be read: ${state.config_error}`);
  }
  if (problem) {
    warnings.push(problem);
  }
  if (!needsRights && warnings.length === 0) {
    return null;
  }

  return (
    <div className="grid gap-2">
      {needsRights ? (
        <div className="flex items-center justify-between gap-3 rounded-island border border-warn/25 bg-warn/10 py-1.5 pr-1.5 pl-3 text-[12px] text-warn">
          <span>Leash needs administrator rights to stop services.</span>
          <button
            type="button"
            onClick={onRestartAsAdmin}
            className="flex-none rounded-island border border-warn/30 px-3 py-1 text-[11.5px] font-medium whitespace-nowrap transition-[background,transform] duration-700 ease-spring hover:bg-warn/15 active:scale-[0.97]"
          >
            Restart as admin
          </button>
        </div>
      ) : null}

      {warnings.map((warning) => (
        <p
          key={warning}
          className="m-0 rounded-island border border-warn/25 bg-warn/10 px-3 py-2 text-[12px] text-warn"
        >
          {warning}
        </p>
      ))}
    </div>
  );
}
