const SECRET =
  /(sk-[A-Za-z0-9]|ghp_|gho_|xox[bp]-|AKIA[0-9A-Z]|BEGIN [A-Z ]*PRIVATE|api[_-]?key\s*[=:]|token\s*[=:]|password\s*[=:]|Bearer\s)/i;

export function isSecret(command: string): boolean {
  return SECRET.test(command);
}

function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Seeded Fisher-Yates shuffle, then the first `count` items. */
export function sample<T>(items: T[], count: number, seed: number): T[] {
  const copy = [...items];
  const random = mulberry32(seed);
  for (let i = copy.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    [copy[i], copy[j]] = [copy[j] as T, copy[i] as T];
  }
  return copy.slice(0, count);
}
