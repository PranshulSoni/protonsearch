// Minimal Node builtin typings for unit tests that read CSS source for
// selector regressions (REVIEW_2 F3). The app bundle never imports Node; the
// project deliberately does not ship @types/node.

declare module "node:fs" {
  export function readFileSync(path: string, encoding: "utf8"): string;
}

declare module "node:url" {
  export function fileURLToPath(url: URL | string): string;
}
