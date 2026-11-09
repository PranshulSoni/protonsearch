/**
 * Brand mark — two-tone geometric P reconstructed from the supplied reference.
 * Light/dark variants come from CSS tokens so the mark stays legible on both
 * surfaces (design: on-dark body #e3e9ee / facet #5b93c4). The counterform is a
 * true cut-out (mask), never a white patch. Source: assets/brand/README.md.
 */
export function Logo({ size = 29, className }: { size?: number; className?: string }) {
  return (
    <svg
      className={className ? "logo " + className : "logo"}
      width={size}
      height={size}
      viewBox="0 0 64 64"
      fill="none"
      aria-label="ProtonSearch"
      role="img"
      shapeRendering="geometricPrecision"
    >
      <defs>
        <mask id="ps-counter" maskUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
          <rect x="0" y="0" width="64" height="64" fill="#fff" />
          <path
            d="M34 18h7.3c4.7 0 5.8 2.9 4.8 6.2-1 3.9-4.6 6.2-10.6 6.2h-4.2L34 18Z"
            fill="#000"
          />
        </mask>
      </defs>
      <g mask="url(#ps-counter)">
        <path
          d="M21.5 5.5h19.8c12.7 0 19.2 8.4 16.2 19.3-2.7 10.3-10.8 16.3-22.9 16.3h-7L21.4 56c-1.7 5.1-6 6.9-10.9 5.2-3.2-1.1-4.9-4-3.9-7.7L16.5 10c.7-2.7 2.3-4.5 5-4.5Z"
          fill="var(--ps-brand-body, #102448)"
        />
        <path
          d="M21.5 5.5h19.8c10.4 0 16.4 5.2 16.7 14.1-9.1-.7-18.5 1.2-27.7 5.3L18.5 41 6.6 53.5 16.5 10c.7-2.7 2.3-4.5 5-4.5Z"
          fill="var(--ps-brand-facet, #1d5189)"
        />
      </g>
    </svg>
  );
}

export function Wordmark({ size = 29 }: { size?: number }) {
  return (
    <span className="brand">
      <Logo size={size} />
      <span>ProtonSearch</span>
    </span>
  );
}
