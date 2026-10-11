// A status tag. Section 11: a soft tinted background, not an outline.

import type { ReactNode } from "react";
import { TAG, TAG_TONES, type Tone } from "../lib/ui";

interface TagProps {
  tone: Tone;
  children: ReactNode;
}

export function Tag({ tone, children }: TagProps) {
  return <span className={`${TAG} ${TAG_TONES[tone]}`}>{children}</span>;
}
