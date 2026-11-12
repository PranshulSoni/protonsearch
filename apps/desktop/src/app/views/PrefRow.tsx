import type { ReactNode } from "react";

/** One settings row: title and optional description on the left, the control on the right. */
export function PrefRow({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <div className="prefs-row">
      <div className="prefs-row-copy">
        <span className="prefs-row-title">{title}</span>
        {description ? <span className="prefs-row-desc">{description}</span> : null}
      </div>
      <div className="prefs-row-control">{children}</div>
    </div>
  );
}
