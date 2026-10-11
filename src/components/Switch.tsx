// The switch from the design reference: a 42 by 24 pill with an 18 pixel knob.

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
      className="relative m-0 h-6 w-[42px] flex-none cursor-pointer appearance-none rounded-pill bg-border transition-colors after:absolute after:top-[3px] after:left-[3px] after:size-[18px] after:rounded-full after:bg-text after:transition-transform checked:bg-accent checked:after:translate-x-[18px] checked:after:bg-white disabled:cursor-default disabled:opacity-50"
    />
  );
}
