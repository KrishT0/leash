// The orbit: the orb, the rings, one dot for each service on a leash, and the
// leash line from each dot to the orb.
//
// The frame loop writes to the DOM rather than to React state, because it runs
// while a spring moves and nothing else. Section 11: only the orb animates all
// the time, and the frame loop runs only while a dot is moving.

import { useCallback, useEffect, useRef } from "react";
import { CAUGHT_RADIUS, dotRadius, dots, point, spring } from "../lib/orbit";
import type { ServiceView } from "../lib/types";

/** The service that was just caught, and when, so the ripple can restart. */
export interface Caught {
  name: string;
  at: number;
}

interface OrbitProps {
  services: ServiceView[];
  caught: Caught | null;
  paused: boolean;
  /** Caught today, for the middle. */
  today: number;
  /** Opens the service sheet for the name, when a dot is clicked. */
  onPick?: (name: string) => void;
}

export function Orbit({ services, caught, paused, today, onPick }: OrbitProps) {
  const radius = dotRadius(services.length);
  const faintLines = services.length > 12 && services.length <= 24;
  const hideLines = services.length > 24;

  /** Where each dot is now, kept across renders so a repaint does not jump. */
  const motion = useRef(new Map<string, { radius: number; velocity: number }>());
  const dotRefs = useRef<(SVGGElement | null)[]>([]);
  const lineRefs = useRef<(SVGLineElement | null)[]>([]);
  const labelRef = useRef<SVGTextElement | null>(null);
  const tipRef = useRef<HTMLDivElement | null>(null);
  /** The name under the pointer, or null. The frame loop reads this. */
  const hoverRef = useRef<string | null>(null);
  /** The frame loop's paint, so a hover can place the tooltip at once. */
  const paintRef = useRef<(() => void) | null>(null);
  const frame = useRef(0);

  useEffect(() => {
    const state = motion.current;
    const layout = dots(services.length);
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    // Forget the services that are gone, and let a new one fly in from the rim.
    const names = new Set(services.map((service) => service.name));
    for (const name of [...state.keys()]) {
      if (!names.has(name)) {
        state.delete(name);
      }
    }
    for (const service of services) {
      if (!state.has(service.name)) {
        state.set(service.name, { radius: CAUGHT_RADIUS, velocity: 0 });
      }
    }

    const targets = layout.map((dot, index) =>
      services[index].name === caught?.name ? CAUGHT_RADIUS : dot.home,
    );

    const paint = () => {
      layout.forEach((dot, index) => {
        const where = state.get(services[index].name);
        const { x, y } = point(dot, where?.radius ?? dot.home);

        dotRefs.current[index]?.setAttribute("transform", `translate(${x} ${y})`);
        lineRefs.current[index]?.setAttribute("x2", String(x));
        lineRefs.current[index]?.setAttribute("y2", String(y));
      });

      // The tooltip follows the pointer, over the dot it belongs to. It comes
      // before the caught label, because that label returns early.
      const tip = tipRef.current;
      const hovered = hoverRef.current;
      const hoverIndex = hovered
        ? services.findIndex((service) => service.name === hovered)
        : -1;
      if (tip) {
        // The caught service already carries its name, so its tooltip stays out
        // of the way.
        if (hoverIndex < 0 || hovered === caught?.name) {
          tip.style.display = "none";
        } else {
          const where = state.get(services[hoverIndex].name);
          const { x, y } = point(
            layout[hoverIndex],
            where?.radius ?? layout[hoverIndex].home,
          );
          tip.style.display = "";
          tip.textContent = services[hoverIndex].display_name;
          tip.style.left = `${x}%`;
          tip.style.top = `${y}%`;
        }
      }

      // Only the service that was just caught gets a name, as section 11 asks.
      const label = labelRef.current;
      const index = services.findIndex((service) => service.name === caught?.name);
      if (!label || index < 0) {
        if (label) label.style.display = "none";
        return;
      }

      const where = state.get(services[index].name);
      const { x, y } = point(layout[index], where?.radius ?? layout[index].home);
      const right = x > 50;
      label.style.display = "";
      label.textContent = services[index].display_name;
      label.setAttribute("x", String(x + (right ? -4 : 4)));
      label.setAttribute("y", String(y + 1));
      label.setAttribute("text-anchor", right ? "end" : "start");
    };

    paintRef.current = paint;

    if (reduced) {
      // No spring, no ripple, no breath.
      layout.forEach((_dot, index) => {
        const where = state.get(services[index].name);
        if (where) {
          where.radius = targets[index];
          where.velocity = 0;
        }
      });
      paint();
      return;
    }

    const step = () => {
      let busy = false;
      layout.forEach((_dot, index) => {
        const where = state.get(services[index].name);
        if (!where) return;

        const next = spring(where.radius, where.velocity, targets[index]);
        where.radius = next.radius;
        where.velocity = next.velocity;
        busy = busy || next.moving;
      });
      paint();
      frame.current = busy ? window.requestAnimationFrame(step) : 0;
    };

    if (!frame.current) {
      frame.current = window.requestAnimationFrame(step);
    }

    return () => {
      paintRef.current = null;
      if (frame.current) {
        window.cancelAnimationFrame(frame.current);
        frame.current = 0;
      }
    };
  }, [services, caught]);

  /** Show the name of the service under the pointer. */
  const enter = useCallback((name: string) => {
    hoverRef.current = name;
    paintRef.current?.();
  }, []);

  /** Hide the name when the pointer leaves. */
  const leave = useCallback(() => {
    hoverRef.current = null;
    paintRef.current?.();
  }, []);

  return (
    <div className="relative mx-auto aspect-square w-full max-w-[330px]">
      <svg
        viewBox="0 0 100 100"
        aria-label="Services on a leash"
        className="absolute inset-0 size-full overflow-visible"
      >
        {[30, 40].map((ring) => (
          <circle
            key={ring}
            cx="50"
            cy="50"
            r={ring}
            fill="none"
            stroke="var(--color-border)"
            strokeWidth={0.25}
            strokeDasharray="0.6 1.6"
          />
        ))}
        <circle
          cx="50"
          cy="50"
          r={47}
          fill="none"
          stroke="color-mix(in srgb, var(--color-warn) 45%, transparent)"
          strokeWidth={0.25}
          strokeDasharray="0.6 1.6"
        />

        <g>
          {services.map((service, index) => {
            const hot = service.name === caught?.name;
            return (
              <line
                key={service.name}
                ref={(element) => {
                  lineRefs.current[index] = element;
                }}
                x1="50"
                y1="50"
                x2="50"
                y2="50"
                stroke={hot ? "var(--color-warn)" : "var(--color-accent)"}
                strokeOpacity={hot ? 0.95 : faintLines ? 0.09 : 0.22}
                strokeWidth={hot ? 0.5 : 0.25}
                strokeDasharray={hot ? "1.6 1" : undefined}
                className={`${hot ? "animate-dash" : ""} ${
                  hideLines && !hot ? "hidden" : ""
                }`}
                style={{ transition: "stroke 0.3s, stroke-opacity 0.3s, stroke-width 0.3s" }}
              />
            );
          })}
        </g>

        <g>
          {services.map((service, index) => (
            /* biome-ignore lint/a11y/useSemanticElements: SVG has no button element, so the role is how a dot says it is a control. */
            <g
              key={service.name}
              ref={(element) => {
                dotRefs.current[index] = element;
              }}
              // A dot is a control, so it answers the keyboard as well as the
              // pointer. Section 11 names the service on hover; focus does the
              // same, so the name is reachable without a mouse.
              role="button"
              tabIndex={0}
              aria-label={service.display_name}
              onClick={() => onPick?.(service.name)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  onPick?.(service.name);
                }
              }}
              onMouseEnter={() => enter(service.name)}
              onMouseLeave={leave}
              onFocus={() => enter(service.name)}
              onBlur={leave}
              className="cursor-pointer"
              style={{
                transition: "fill 0.3s",
                filter:
                  service.name === caught?.name
                    ? "drop-shadow(0 0 2px var(--color-warn))"
                    : "drop-shadow(0 0 1.2px var(--color-accent))",
              }}
            >
              {/*
               * A dot is only a few pixels wide, so a slightly larger circle
               * takes the pointer. It stays close to the dot, or the hand would
               * appear over empty space. A transparent fill still receives
               * events; no fill does not.
               */}
              <circle r={radius + 1.6} fill="transparent" />
              <circle
                r={radius}
                fill={service.name === caught?.name ? "var(--color-warn)" : "var(--color-accent)"}
                className="pointer-events-none"
              />
            </g>
          ))}
        </g>

        <text
          ref={labelRef}
          fontSize={3.4}
          fill="var(--color-text)"
          stroke="var(--color-bg)"
          strokeWidth={0.8}
          paintOrder="stroke"
          className="pointer-events-none font-display"
        />
      </svg>

      {/*
       * Starts hidden through an inline style, not a class: the frame loop
       * shows it again by clearing the style, which a class would override.
       */}
      <div
        ref={tipRef}
        style={{ display: "none" }}
        className="pointer-events-none absolute -translate-x-1/2 -translate-y-[calc(100%+10px)] rounded-island border border-border bg-panel px-2 py-1 text-[10.5px] font-medium whitespace-nowrap text-text shadow-[0_12px_34px_-14px_rgba(0,0,0,0.8)]"
      />

      <div className="pointer-events-none absolute left-1/2 top-1/2 aspect-square w-[30%] -translate-x-1/2 -translate-y-1/2">
        {caught ? (
          <div
            key={caught.at}
            className="size-full animate-ripple rounded-full border-2 border-accent"
          />
        ) : null}
      </div>

      <div className="absolute left-1/2 top-1/2 aspect-square w-[30%] -translate-x-1/2 -translate-y-1/2">
        <div
          className={`grid size-full place-content-center rounded-full text-center ${
            paused ? "grayscale brightness-75" : "animate-breathe"
          }`}
          style={{
            background:
              "radial-gradient(circle at 50% 36%, color-mix(in srgb, var(--color-accent) 42%, var(--color-surface)) 0, var(--color-surface) 66%)",
            boxShadow:
              "0 0 0 1px var(--color-border), 0 0 70px color-mix(in srgb, var(--color-accent) 30%, transparent)",
          }}
        >
          <div>
            <div
              key={today}
              className="animate-bump text-[min(9vw,38px)] leading-none font-bold tracking-[-0.05em]"
            >
              {today}
            </div>
            <div className="mt-0.5 text-[10px] text-muted">caught today</div>
          </div>
        </div>
      </div>
    </div>
  );
}
