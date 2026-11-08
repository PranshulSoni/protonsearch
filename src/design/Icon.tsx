import sprite from "./icons.svg?url";
export function Icon({
  name,
  className = "",
}: {
  name: string;
  className?: string;
}) {
  return (
    <svg
      className={`dock-glyph ${className}`}
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <use href={`${sprite}#dock-icon-${name}`} />
    </svg>
  );
}
