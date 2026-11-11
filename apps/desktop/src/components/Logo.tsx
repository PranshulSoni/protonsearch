/**
 * Brand mark — authoritative silver/red ornate P variant.
 * Transparent P for Dynamic Island, in-app sidebar/header, about view.
 * App tile variant (charcoal rounded square backing) for app tile contexts.
 */
export function Logo({
  size = 29,
  className,
  variant = "transparent",
}: {
  size?: number;
  className?: string;
  variant?: "transparent" | "tile";
}) {
  const filePrefix = variant === "tile" ? "app-tile" : "transparent-p";
  const src =
    size <= 20
      ? `/brand/${filePrefix}-20.png`
      : size <= 32
        ? `/brand/${filePrefix}-32.png`
        : size <= 64
          ? `/brand/${filePrefix}-64.png`
          : size <= 128
            ? `/brand/${filePrefix}-128.png`
            : `/brand/${filePrefix}-256.png`;

  return (
    <img
      src={src}
      srcSet={`/brand/${filePrefix}-32.png 1x, /brand/${filePrefix}-64.png 2x, /brand/${filePrefix}-128.png 3x`}
      width={size}
      height={size}
      alt="ProtonSearch"
      className={className ? "logo " + className : "logo"}
      style={{
        display: "inline-block",
        verticalAlign: "middle",
        objectFit: "contain",
      }}
      draggable={false}
    />
  );
}

export function AppTileLogo({ size = 32, className }: { size?: number; className?: string }) {
  return <Logo size={size} className={className} variant="tile" />;
}

export function Wordmark({ size = 29 }: { size?: number }) {
  return (
    <span className="brand">
      <Logo size={size} />
      <span>ProtonSearch</span>
    </span>
  );
}
