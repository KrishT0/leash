// Services. Section 11: a search, three views, a filter sheet, an add sheet, and
// a sheet for every service.

import { useCallback, useEffect, useState } from "react";
import { AddSheet } from "../components/AddSheet";
import { Bezel } from "../components/Bezel";
import { FilterSheet } from "../components/FilterSheet";
import { Segmented } from "../components/Segmented";
import { ServiceRow } from "../components/ServiceRow";
import { ServiceSheet } from "../components/ServiceSheet";
import { listServices } from "../lib/api";
import {
  DEFAULT_FILTERS,
  TAB_LABELS,
  activeFilters,
  filterServices,
  type Filters,
  type ServiceTab,
} from "../lib/services";
import type { ServiceView } from "../lib/types";
import { EYEBROW, GHOST } from "../lib/ui";

const TABS = (Object.keys(TAB_LABELS) as ServiceTab[]).map((value) => ({
  value,
  label: TAB_LABELS[value],
}));

interface ServicesProps {
  /** Called after anything that changes the config, so the app catches up. */
  onChanged: () => void;
}

export function Services({ onChanged }: ServicesProps) {
  const [services, setServices] = useState<ServiceView[] | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [tab, setTab] = useState<ServiceTab>("suggested");
  const [query, setQuery] = useState("");
  const [filters, setFilters] = useState<Filters>(DEFAULT_FILTERS);
  const [selected, setSelected] = useState<string | null>(null);
  const [filtering, setFiltering] = useState(false);
  const [adding, setAdding] = useState(false);

  const load = useCallback(async () => {
    try {
      setServices(await listServices());
      setProblem(null);
    } catch (error) {
      setProblem(String(error));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  // A change from a sheet has to show in the list too.
  const changed = useCallback(() => {
    void load();
    onChanged();
  }, [load, onChanged]);

  const rows = services ? filterServices(services, tab, query, filters) : null;
  const filterCount = activeFilters(filters);

  return (
    <section>
      <Bezel coreClassName="gap-5">
        <div className="flex items-center justify-between gap-3">
          <p className={`m-0 ${EYEBROW}`}>Find services</p>
          <span className="flex gap-2">
            <button type="button" className={GHOST} onClick={() => setFiltering(true)}>
              Filters{filterCount > 0 ? ` ${filterCount}` : ""}
            </button>
            <button type="button" className={GHOST} onClick={() => setAdding(true)}>
              Add
            </button>
          </span>
        </div>

        <Segmented options={TABS} value={tab} onChange={setTab} label="View" />

        <input
          type="search"
          value={query}
          onChange={(event) => setQuery(event.currentTarget.value)}
          placeholder="Search by name"
          aria-label="Search services"
          className="w-full rounded-island border border-hair bg-bg px-4 py-3 text-sm text-text placeholder:text-muted"
        />

        {problem ? <p className="m-0 text-[13px] text-error">{problem}</p> : null}

        {rows === null ? (
          <p className="m-0 text-[13px] text-muted">Reading the services.</p>
        ) : rows.length === 0 ? (
          <p className="m-0 text-[13px] text-muted">
            No match. Try the internal name or reset the filters.
          </p>
        ) : (
          <div>
            {rows.map((service) => (
              <ServiceRow
                key={service.name}
                service={service}
                showReason={tab === "suggested"}
                onOpen={() => setSelected(service.name)}
              />
            ))}
          </div>
        )}
      </Bezel>

      <FilterSheet
        open={filtering}
        filters={filters}
        count={rows?.length ?? 0}
        onChange={setFilters}
        onClose={() => setFiltering(false)}
      />

      <AddSheet
        open={adding}
        services={services ?? []}
        onClose={() => setAdding(false)}
        onAdded={changed}
      />

      <ServiceSheet name={selected} onClose={() => setSelected(null)} onChanged={changed} />
    </section>
  );
}
