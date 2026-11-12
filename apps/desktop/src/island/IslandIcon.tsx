import type { SVGProps } from "react";

/**
 * Island-scoped inline icons: 24 viewBox, fill none, stroke currentColor,
 * stroke-width 1.85, round caps/joins (ISLAND_V9_SPEC §1.4). Never upscale:
 * render at 14/15/16/18/19/20/23/24 only.
 */

export type IslandIconName =
  | "search"
  | "media"
  | "clipboard"
  | "agent"
  | "controls"
  | "browser"
  | "git"
  | "bell"
  | "back"
  | "chevron"
  | "more"
  | "check"
  | "dismiss"
  | "plus"
  | "play"
  | "pause"
  | "next"
  | "prev"
  | "shuffle"
  | "repeat"
  | "output"
  | "volume"
  | "brightness"
  | "wifi"
  | "bluetooth"
  | "airplane"
  | "bolt"
  | "file"
  | "link"
  | "code"
  | "image"
  | "text"
  | "pin"
  | "pause-circle";

const PATHS: Record<IslandIconName, string[]> = {
  search: ["M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16Z", "m21 21-4.35-4.35"],
  media: ["M9 18V5l12-2v13", "M9 18a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z", "M21 16a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z"],
  clipboard: ["M9 5H7a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-2", "M9 3h6v4H9z"],
  agent: ["M12 2v3", "M6 8h12a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2Z", "M9 13h.01", "M15 13h.01", "M9 17h6"],
  controls: ["M4 21v-7", "M4 10V3", "M12 21v-9", "M12 8V3", "M20 21v-5", "M20 12V3", "M1 14h6", "M9 8h6", "M17 16h6"],
  browser: ["M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20Z", "M2 12h20", "M12 2a15 15 0 0 1 0 20 15 15 0 0 1 0-20Z"],
  git: ["M6 3v12", "M18 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z", "M6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z", "M18 9a9 9 0 0 1-9 9"],
  bell: ["M18 8a6 6 0 0 0-12 0c0 7-3 9-3 9h18s-3-2-3-9", "M13.7 21a2 2 0 0 1-3.4 0"],
  back: ["m15 18-6-6 6-6"],
  chevron: ["m9 18 6-6-6-6"],
  more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
  check: ["m20 6-11 11-5-5"],
  dismiss: ["M18 6 6 18", "m6 6 12 12"],
  plus: ["M12 5v14", "M5 12h14"],
  play: ["m7 4 13 8-13 8V4Z"],
  pause: ["M8 4v16", "M16 4v16"],
  next: ["m5 4 10 8-10 8V4Z", "M19 5v14"],
  prev: ["m19 4-10 8 10 8V4Z", "M5 5v14"],
  shuffle: ["M16 3h5v5", "M4 20 21 3", "M21 16v5h-5", "M15 15l6 6", "M4 4l5 5"],
  repeat: ["m17 2 4 4-4 4", "M3 11v-1a4 4 0 0 1 4-4h14", "m7 22-4-4 4-4", "M21 13v1a4 4 0 0 1-4 4H3"],
  output: ["M4 17V7a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v10", "M2 21h20", "M8 11h8"],
  volume: ["M11 5 6 9H2v6h4l5 4V5Z", "M15.5 8.5a5 5 0 0 1 0 7", "M18.5 5.5a9 9 0 0 1 0 13"],
  brightness: ["M12 17a5 5 0 1 0 0-10 5 5 0 0 0 0 10Z", "M12 1v2", "M12 21v2", "M4.2 4.2l1.4 1.4", "M18.4 18.4l1.4 1.4", "M1 12h2", "M21 12h2", "M4.2 19.8l1.4-1.4", "M18.4 5.6l1.4-1.4"],
  wifi: ["M5 12.5a10 10 0 0 1 14 0", "M8.5 16a5 5 0 0 1 7 0", "M12 19.5h.01"],
  bluetooth: ["m7 7 10 10-5 5V2l5 5L7 17"],
  airplane: ["M17.8 19.2 16 11l4.4-4.4a2 2 0 0 0-2.8-2.8L13 8.2 4.8 6.4a1 1 0 0 0-1 1.6l5.3 4.3-2.2 4.1a1 1 0 0 0 1.6 1.1l3.4-2.2 4.2 5.3a1 1 0 0 0 1.7-.6Z"],
  bolt: ["m13 2-9 12h7l-1 8 9-12h-7l1-8Z"],
  file: ["M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8Z", "M14 2v6h6"],
  link: ["M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-2 2", "M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l2-2"],
  code: ["m16 18 6-6-6-6", "m8 6-6 6 6 6"],
  image: ["M21 3H3v18h18V3Z", "m3 16 5-5 4 4 3-3 6 6", "M9 9h.01"],
  text: ["M4 7V5h16v2", "M9 19h6", "M12 5v14"],
  pin: ["M12 17v5", "M9 3h6l-1 8 4 4H6l4-4-1-8Z"],
  "pause-circle": ["M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20Z", "M10 9v6", "M14 9v6"],
};

export interface IslandIconProps extends Omit<SVGProps<SVGSVGElement>, "name"> {
  name: IslandIconName;
  size?: number;
}

export function IslandIcon({ name, size = 19, ...rest }: IslandIconProps) {
  return (
    <svg
      {...rest}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.85}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {PATHS[name].map((d) => (
        <path d={d} key={d} />
      ))}
    </svg>
  );
}
