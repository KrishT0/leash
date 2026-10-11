// A group of content: one surface, one hairline, one corner.
//
// Cards never nest. A card inside a card reads as an app inside an app, so this
// stays a single layer, and an item inside a card uses the control fill.

import type { ReactNode } from "react";

interface CardProps {
  children: ReactNode;
  /** True for the one card that carries the accent glow, which is the counter. */
  glow?: boolean;
  /** Placement only, such as a grid span. */
  className?: string;
}

export function Card({ children, glow, className = "" }: CardProps) {
  return (
    <div
      className={`grid content-start gap-4 rounded-card border border-hair bg-surface p-6 shadow-card ${className}`}
      style={
        glow
          ? {
              background:
                "radial-gradient(closest-side, color-mix(in srgb, var(--color-accent) 16%, transparent), transparent 72%) 50% 0%/360px 260px no-repeat, var(--color-surface)",
            }
          : undefined
      }
    >
      {children}
    </div>
  );
}
