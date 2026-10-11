import { describe, expect, it } from "vitest";
import { CAUGHT_RADIUS, dotRadius, dots, point, spring } from "./orbit";

describe("dots", () => {
  it("should put no dots on an empty orbit", () => {
    expect(dots(0)).toEqual([]);
  });

  it("should start the first dot at angle zero", () => {
    const [first] = dots(4);
    expect(first.angle).toBe(0);
  });

  it("should turn each dot by 137.5 degrees", () => {
    const [, second] = dots(4);
    expect(second.angle).toBeCloseTo((137.5 * Math.PI) / 180, 10);
  });

  it("should use the sunflower radius from the instructions", () => {
    // 22 + 20 * sqrt((k + 0.5) / N), for the first of one.
    expect(dots(1)[0].home).toBeCloseTo(22 + 20 * Math.sqrt(0.5), 10);
    // The fourth of eight.
    expect(dots(8)[3].home).toBeCloseTo(22 + 20 * Math.sqrt(3.5 / 8), 10);
  });

  it("should keep every dot inside the edge ring", () => {
    for (const count of [1, 5, 12, 30, 60]) {
      for (const dot of dots(count)) {
        expect(dot.home).toBeLessThan(CAUGHT_RADIUS);
      }
    }
  });
});

describe("dotRadius", () => {
  it("should use the larger dot up to twelve services", () => {
    expect(dotRadius(1)).toBe(2.2);
    expect(dotRadius(12)).toBe(2.2);
  });

  it("should use the smaller dot above twelve", () => {
    expect(dotRadius(13)).toBe(1.3);
    expect(dotRadius(40)).toBe(1.3);
  });
});

describe("point", () => {
  it("should put the angle zero dot on the right", () => {
    expect(point({ angle: 0, home: 40 }, 40)).toEqual({ x: 90, y: 50 });
  });

  it("should turn a quarter turn down the screen", () => {
    const { x, y } = point({ angle: Math.PI / 2, home: 40 }, 40);
    expect(x).toBeCloseTo(50, 10);
    expect(y).toBeCloseTo(90, 10);
  });
});

describe("spring", () => {
  it("should pull a dot home", () => {
    const moving = spring(47, 0, 30);
    expect(moving.radius).toBeLessThan(47);
    expect(moving.moving).toBe(true);
  });

  it("should sit still when it is already home", () => {
    const still = spring(30, 0, 30);
    expect(still).toEqual({ radius: 30, velocity: 0, moving: false });
  });

  it("should settle at the target and stop", () => {
    let current = 47;
    let velocity = 0;
    let frames = 0;

    // The frame loop must always end, or it would run for ever.
    while (frames < 600) {
      const next = spring(current, velocity, 30);
      current = next.radius;
      velocity = next.velocity;
      frames++;
      if (!next.moving) break;
    }

    expect(frames).toBeLessThan(600);
    expect(current).toBe(30);
    expect(velocity).toBe(0);
  });

  it("should bounce past the target before settling, as a spring does", () => {
    let current = CAUGHT_RADIUS;
    let velocity = 0;
    let smallest = current;

    for (let step = 0; step < 30; step++) {
      const next = spring(current, velocity, 30);
      current = next.radius;
      velocity = next.velocity;
      smallest = Math.min(smallest, current);
    }

    expect(smallest).toBeLessThan(30);
  });
});
