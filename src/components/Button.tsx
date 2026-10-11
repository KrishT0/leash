// The primary action. The design skill asks for a pill with the trailing icon
// nested in its own circle, which moves on hover, so this is a component rather
// than a class string.

import type { ReactNode } from "react";

interface CtaProps {
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  type?: "button" | "submit";
  /** The glyph in the nested circle. An arrow unless something else fits. */
  glyph?: string;
}

export function Cta({ children, onClick, disabled, type = "button", glyph = "↗" }: CtaProps) {
  return (
    <button
      type={type}
      onClick={onClick}
      disabled={disabled}
      className="inline-flex items-center gap-3.5 rounded-island bg-text py-2.5 pr-2.5 pl-5 text-sm font-semibold text-bg transition-[transform,box-shadow] duration-700 ease-spring hover:shadow-[0_18px_40px_-12px_var(--color-border)] active:scale-[0.98] disabled:opacity-40"
    >
      {children}
      <span
        aria-hidden="true"
        className="grid size-8 flex-none place-items-center rounded-island bg-bg/10 transition-transform duration-700 ease-spring group-hover:translate-x-0.5"
      >
        {glyph}
      </span>
    </button>
  );
}
