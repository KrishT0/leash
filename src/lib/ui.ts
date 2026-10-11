// The button and tag classes, in one place so a page cannot drift from the
// design. Tailwind reads these strings from this file, so the utilities are
// generated even though they are only written here.

/** The primary action. Section 11: an 8px corner and the accent colour. */
export const PRIMARY =
  "rounded-button border-0 bg-accent px-[18px] py-[9px] text-sm font-semibold text-white active:scale-[0.98] disabled:opacity-50";

/** A quiet action. Section 11: a pill with a hairline border. */
export const GHOST =
  "rounded-pill border border-border bg-none px-3.5 py-[5px] text-xs font-medium text-muted hover:border-accent hover:text-text active:scale-[0.98]";

/** A status tag: a soft tinted background, never an outline. Section 11. */
export const TAG_TONES = {
  ok: "bg-ok/13 text-ok",
  warn: "bg-warn/13 text-warn",
  error: "bg-error/13 text-error",
  neutral: "bg-border/40 text-muted",
} as const;

/** The tag shape, with no colour of its own. */
export const TAG = "ml-1.5 inline-block rounded-pill px-2 py-px font-mono text-[11px]";
