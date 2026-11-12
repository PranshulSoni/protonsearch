import { cn } from "@/lib/utils";

/**
 * Horizontal slider. A native range input does the keyboard, pointer and
 * screen-reader work; track, fill and thumb are drawn around it.
 */
export function Slider({
  value,
  onChange,
  label,
  min = 0,
  max = 100,
  step = 1,
  disabled = false,
  className,
}: {
  value: number;
  onChange: (value: number) => void;
  label: string;
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
  className?: string;
}) {
  const percent = max > min ? Math.min(100, Math.max(0, ((value - min) / (max - min)) * 100)) : 0;
  return (
    <div className={cn("relative flex h-5 items-center", disabled && "opacity-50", className)}>
      <div className="absolute inset-x-0 h-1 rounded-full bg-raised" />
      {/* Fill and thumb move on transform only: no layout work while the value changes. */}
      <div
        className="absolute inset-x-0 h-1 origin-left rounded-full bg-hi motion-safe:transition-transform motion-safe:duration-150"
        style={{ transform: `scaleX(${percent / 100})` }}
      />
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(Number(event.target.value))}
        // A zero-width native thumb maps the pointer straight to value = x / width, the same
        // geometry the drawn fill and thumb use, so the value lands exactly where you click.
        className="peer absolute inset-0 m-0 size-full cursor-pointer appearance-none bg-transparent opacity-0 disabled:cursor-default [&::-webkit-slider-thumb]:size-0 [&::-webkit-slider-thumb]:appearance-none"
      />
      <div
        className="pointer-events-none absolute inset-0 motion-safe:transition-transform motion-safe:duration-150 peer-focus-visible:*:ring-2 peer-focus-visible:*:ring-hi"
        style={{ transform: `translateX(${percent}%)` }}
      >
        <div className="ps-thumb absolute top-1/2 left-0 size-3 -translate-x-1/2 -translate-y-1/2 rounded-full bg-fg" />
      </div>
    </div>
  );
}
