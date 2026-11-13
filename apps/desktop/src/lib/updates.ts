// Update check: asks GitHub for the latest release (only when the user clicks)
// and compares its tag with the running version. The CSP allows this host.

const LATEST_RELEASE_API = "https://api.github.com/repos/PranshulSoni/protonsearch/releases/latest";
export const RELEASES_PAGE = "https://github.com/PranshulSoni/protonsearch/releases/latest";

export type UpdateCheck =
  | { kind: "current" }
  | { kind: "available"; version: string }
  | { kind: "error"; text: string };

function parseVersion(version: string): { core: number[]; pre: string } {
  const clean = version.trim().replace(/^v/i, "").split("+")[0];
  const dash = clean.indexOf("-");
  const core = (dash < 0 ? clean : clean.slice(0, dash)).split(".").map((part) => Number.parseInt(part, 10) || 0);
  return { core, pre: dash < 0 ? "" : clean.slice(dash + 1) };
}

/** Semver order of two versions; a leading "v" is ignored. Negative when a is older. */
export function compareVersions(a: string, b: string): number {
  const x = parseVersion(a);
  const y = parseVersion(b);
  for (let i = 0; i < 3; i += 1) {
    const diff = (x.core[i] ?? 0) - (y.core[i] ?? 0);
    if (diff !== 0) return diff;
  }
  if (x.pre === y.pre) return 0;
  if (!x.pre) return 1; // a release outranks its prereleases
  if (!y.pre) return -1;
  // ponytail: natural order for prerelease tags (alpha.2 < alpha.10 < beta), not every semver identifier rule
  return x.pre.localeCompare(y.pre, undefined, { numeric: true });
}

export async function checkForUpdate(current: string): Promise<UpdateCheck> {
  try {
    const response = await fetch(LATEST_RELEASE_API);
    if (response.status === 404) return { kind: "error", text: "No releases published yet." };
    const release = (await response.json()) as { tag_name?: string };
    if (!response.ok || !release.tag_name) throw new Error("No release tag");
    return compareVersions(release.tag_name, current) > 0
      ? { kind: "available", version: release.tag_name.replace(/^v/i, "") }
      : { kind: "current" };
  } catch {
    return { kind: "error", text: "Couldn't check for updates. Check your connection." };
  }
}
