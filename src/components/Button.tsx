// The primary action: a solid pill. The design skill nests an icon in its own
// circle, but the label alone reads cleaner at this size, so the pill carries
// the words and nothing else.

import type { ReactNode } from "react";

interface CtaProps {
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  type?: "button" | "submit";
}

export function Cta({ children, onClick, disabled, type = "button" }: CtaProps) {
  return (
    <button
      type={type}
      onClick={onClick}
      disabled={disabled}
      className="inline-flex items-center rounded-island bg-text px-4 py-1.5 text-[12px] font-semibold text-bg transition-[transform,box-shadow] duration-700 ease-spring hover:shadow-[0_14px_30px_-12px_var(--color-border)] active:scale-[0.98] disabled:cursor-default disabled:opacity-40"
    >
      {children}
    </button>
  );
}
