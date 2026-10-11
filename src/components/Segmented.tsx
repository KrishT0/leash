// The segmented control from the design reference: a pill of buttons with the
// chosen one filled in.

interface Option<T extends string> {
  value: T;
  label: string;
}

interface SegmentedProps<T extends string> {
  options: Option<T>[];
  value: T;
  onChange: (value: T) => void;
  label: string;
  disabled?: boolean;
}

export function Segmented<T extends string>({
  options,
  value,
  onChange,
  label,
  disabled,
}: SegmentedProps<T>) {
  return (
    <div
      role="group"
      aria-label={label}
      className="my-2.5 flex rounded-pill border border-border bg-bg p-[3px]"
    >
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          disabled={disabled}
          aria-pressed={option.value === value}
          onClick={() => onChange(option.value)}
          className={`flex-1 rounded-pill p-[7px] text-[13px] font-medium disabled:cursor-default disabled:opacity-40 ${
            option.value === value ? "bg-border text-text" : "text-muted"
          }`}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
