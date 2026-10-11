// A status tag. Section 11: a soft tinted background, not an outline.

import type { ReactNode } from "react";
import { TAG, TAG_TONES } from "../lib/ui";

interface TagProps {
  tone: keyof typeof TAG_TONES;
  children: ReactNode;
}

export function Tag({ tone, children }: TagProps) {
  return <span className={`${TAG} ${TAG_TONES[tone]}`}>{children}</span>;
}
