// The switch from the design language: a wider pill with a white knob, filled
// with the accent when it is on.

interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  /** Read out by a screen reader, because the visible label sits elsewhere. */
  label: string;
  disabled?: boolean;
}

export function Switch({ checked, onChange, label, disabled }: SwitchProps) {
  return (
    <input
      type="checkbox"
      role="switch"
      aria-label={label}
      checked={checked}
      disabled={disabled}
      onChange={(event) => onChange(event.currentTarget.checked)}
      className="relative m-0 h-[26px] w-12 flex-none cursor-pointer appearance-none rounded-island border border-hair bg-border transition-colors duration-700 ease-spring after:absolute after:top-[3px] after:left-[3px] after:size-[18px] after:rounded-island after:bg-text after:transition-transform after:duration-700 after:ease-spring checked:bg-accent/55 checked:after:translate-x-[22px] disabled:cursor-default disabled:opacity-50"
    />
  );
}
