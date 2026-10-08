import type { Resource } from "i18next";

export const BRAND_NAME = "Bachuc";

// Matches `{{...}}` placeholders too, so a placeholder name never changes.
const UPSTREAM_NAME = /\{\{[^}]*\}\}|\bPaseo\b/g;

interface BrandedStrings {
  [key: string]: string | BrandedStrings;
}

export function brandText(text: string): string {
  return text.replace(UPSTREAM_NAME, (match) => (match === "Paseo" ? BRAND_NAME : match));
}

function brandStrings(tree: object): BrandedStrings {
  const branded: BrandedStrings = {};
  for (const [key, value] of Object.entries(tree)) {
    if (typeof value === "string") {
      branded[key] = brandText(value);
    } else if (typeof value === "object" && value !== null) {
      branded[key] = brandStrings(value);
    }
  }
  return branded;
}

// Brands resource strings only, so a value passed to `t()` for a placeholder keeps "Paseo".
export function withBrandName(resources: Resource): Resource {
  return Object.fromEntries(
    Object.entries(resources).map(([language, namespaces]) => [language, brandStrings(namespaces)]),
  );
}
