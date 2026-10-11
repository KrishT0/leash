// The segmented control from the design language: a frosted track of pills.

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
    <fieldset
      aria-label={label}
      className="m-0 my-2 flex min-w-0 gap-1 rounded-island border border-hair border-solid bg-control p-1"
    >
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          disabled={disabled}
          aria-pressed={option.value === value}
          onClick={() => onChange(option.value)}
          className={`flex-1 rounded-island p-1.5 text-[12px] font-medium transition-[background,color] duration-700 ease-spring disabled:cursor-default disabled:opacity-40 ${
            option.value === value ? "bg-border text-text" : "text-muted hover:text-text"
          }`}
        >
          {option.label}
        </button>
      ))}
    </fieldset>
  );
}
