// The card language: an aluminium tray holding a glass plate.
//
// Every card is a shell with its own hairline and padding, and inside it a core
// plate with its own corner and its own shadow. The design skill calls this the
// double-bezel, and it is what makes a panel read as hardware rather than as a
// rectangle with a border.

import type { ReactNode } from "react";

interface BezelProps {
  children: ReactNode;
  /** True for the one card that carries the accent glow, which is the hero. */
  glow?: boolean;
  className?: string;
  /** Padding of the inner plate, when a card needs a different rhythm. */
  coreClassName?: string;
}

export function Bezel({ children, glow, className = "", coreClassName = "" }: BezelProps) {
  return (
    <div className={`rounded-bezel border border-hair bg-shell p-[7px] ${className}`}>
      <div
        className={`grid h-full content-start gap-4 rounded-core bg-surface p-6 shadow-core ${coreClassName}`}
        style={
          glow
            ? {
                background:
                  "radial-gradient(closest-side, color-mix(in srgb, var(--color-accent) 20%, transparent), transparent 72%) 50% 0%/340px 260px no-repeat, var(--color-surface)",
              }
            : undefined
        }
      >
        {children}
      </div>
    </div>
  );
}
