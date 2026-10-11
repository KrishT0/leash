// Services. Section 11: a search, three views, a filter sheet, an add sheet, and
// a sheet for every service.

import { useCallback, useEffect, useState } from "react";
import { AddSheet } from "../components/AddSheet";
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
import { GHOST } from "../lib/ui";

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
      <h2 className="mt-0 mb-2.5 flex items-center justify-between text-sm font-medium text-muted">
        <span>Find services</span>
        <span className="flex gap-1.5">
          <button type="button" className={GHOST} onClick={() => setFiltering(true)}>
            Filters{filterCount > 0 ? ` ${filterCount}` : ""}
          </button>
          <button type="button" className={GHOST} onClick={() => setAdding(true)}>
            Add
          </button>
        </span>
      </h2>

      <Segmented options={TABS} value={tab} onChange={setTab} label="View" />

      <input
        type="search"
        value={query}
        onChange={(event) => setQuery(event.currentTarget.value)}
        placeholder="Search by name"
        aria-label="Search services"
        className="my-1.5 w-full rounded-pill border border-border bg-surface px-4 py-2.5 text-text"
      />

      {problem ? <p className="my-2 text-[13px] text-error">{problem}</p> : null}

      {rows === null ? (
        <p className="text-muted">Reading the services.</p>
      ) : rows.length === 0 ? (
        <p className="text-muted">No match. Try the internal name or reset the filters.</p>
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
