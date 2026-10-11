// The classes that carry the design language, in one place so no page drifts.
// Tailwind reads these strings from this file, so the utilities are generated
// even though they are only written here.

/** A quiet action: a frosted pill. */
export const GHOST =
  "inline-flex items-center gap-2 rounded-island border border-border bg-control px-3.5 py-1.5 text-[12px] font-medium text-text transition-[background,transform] duration-700 ease-spring hover:bg-border active:scale-[0.98] disabled:opacity-40";

/** A status tag: a tinted pill, small and tracked. */
export const TAG_TONES = {
  ok: "bg-ok/12 text-ok",
  warn: "bg-warn/12 text-warn",
  error: "bg-error/12 text-error",
  accent: "bg-accent/16 text-text",
  dim: "bg-border text-muted",
} as const;

/** The tone names, so a module can name one without pulling in the classes. */
export type Tone = keyof typeof TAG_TONES;

/** The tag shape, with no colour of its own. */
export const TAG =
  "inline-block rounded-island px-1.5 py-0.5 font-display text-[9px] leading-relaxed font-medium tracking-[0.12em] whitespace-nowrap uppercase";

/** The eyebrow label that precedes a heading or sits over a card. */
export const EYEBROW =
  "font-display text-[9px] font-medium tracking-[0.2em] uppercase text-muted";

/** A small line of machine text, for numbers and paths. */
export const MACHINE = "font-mono text-[11px] leading-relaxed text-muted";
