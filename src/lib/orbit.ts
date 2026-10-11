// The orbit maths, kept apart from the drawing so it can be tested.

/** Above this many services, the leash lines go faint. Section 11. */
export const FULL_LINES_UP_TO = 12;

/** Above this many, the lines are hidden, except the one that was caught. */
export const FAINT_LINES_UP_TO = 24;

/** The radius a dot flies out to when it is caught. */
export const CAUGHT_RADIUS = 47;

export interface Dot {
  /** The angle from the middle, in radians. */
  angle: number;
  /** The radius the dot rests at. */
  home: number;
}

/**
 * Where each dot sits.
 *
 * Section 11 asks for a sunflower: the angle is the index times 137.5 degrees,
 * and the radius is `22 + 20 * sqrt((k + 0.5) / N)`, which spreads any number of
 * services evenly over the disc.
 */
export function dots(count: number): Dot[] {
  if (count <= 0) {
    return [];
  }

  return Array.from({ length: count }, (_, index) => ({
    angle: (index * 137.5 * Math.PI) / 180,
    home: 22 + 20 * Math.sqrt((index + 0.5) / count),
  }));
}

/** The dot radius. Dots get smaller once there are many services. */
export function dotRadius(count: number): number {
  return count > FULL_LINES_UP_TO ? 1.3 : 2.2;
}

/** The point on the circle, in the 0 to 100 space of the viewBox. */
export function point(dot: Dot, radius: number): { x: number; y: number } {
  return {
    x: 50 + radius * Math.cos(dot.angle),
    y: 50 + radius * Math.sin(dot.angle),
  };
}

export interface Spring {
  radius: number;
  velocity: number;
  /** True while the dot is still moving, which is when the frame loop runs. */
  moving: boolean;
}

/**
 * One step of the spring that pulls a dot home, from section 11:
 * `v = (v + (target - r) * 0.12) * 0.8`.
 */
export function spring(current: number, velocity: number, target: number): Spring {
  const nextVelocity = (velocity + (target - current) * 0.12) * 0.8;
  const nextRadius = current + nextVelocity;
  const moving =
    Math.abs(target - nextRadius) > 0.05 || Math.abs(nextVelocity) > 0.05;

  return moving
    ? { radius: nextRadius, velocity: nextVelocity, moving }
    : { radius: target, velocity: 0, moving: false };
}
