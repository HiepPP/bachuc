import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

const repair = z.string().trim().min(1).max(300);
const label = z.string().trim().min(1).max(80);
const relativePath = z
  .string()
  .min(1)
  .max(240)
  .refine(
    (value) =>
      !value.startsWith("/") &&
      !value.includes("\\") &&
      !value.split("/").includes("..") &&
      !value.includes(":"),
    "Use a relative path inside the workspace.",
  );
const localUrl = z
  .string()
  .max(500)
  .refine((value) => {
    try {
      const url = new URL(value);
      return (
        ["http:", "https:"].includes(url.protocol) &&
        ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname) &&
        !url.username &&
        !url.password &&
        !url.search &&
        !url.hash
      );
    } catch {
      return false;
    }
  }, "Use a loopback HTTP(S) URL without credentials, query, or fragment.");

export const configSchema = z.strictObject({
  version: z.literal(1),
  runtimes: z
    .array(
      z
        .strictObject({
          executable: z.enum(["node", "python3", "bun", "deno"]),
          major: z.number().int().min(1).max(999).optional(),
          range: z.string().trim().min(1).max(200).optional(),
          repair,
        })
        .refine(
          (item) => (item.major === undefined) !== (item.range === undefined),
          "Use exactly one of major or range.",
        ),
    )
    .max(6)
    .optional(),
  dependencies: z
    .array(z.strictObject({ label, path: relativePath, repair }))
    .max(6)
    .optional(),
  ports: z
    .array(
      z.strictObject({
        label,
        port: z.number().int().min(1).max(65535),
        expect: z.enum(["listening", "closed"]),
        repair,
      }),
    )
    .max(6)
    .optional(),
  health: z
    .array(
      z.strictObject({
        label,
        url: localUrl,
        status: z.number().int().min(200).max(599).default(200),
        repair,
      }),
    )
    .max(6)
    .optional(),
});
export type Config = z.infer<typeof configSchema>;
export const measurementSchema = z.object({
  label: z.string(),
  status: z.enum(["pass", "blocker", "unknown"]),
  detail: z.string(),
  repair: z.string(),
});
export type Measurement = z.infer<typeof measurementSchema>;
export const categorySchema = z.enum([
  "configuration",
  "runtimes",
  "dependencies",
  "ports",
  "health",
]);
export type Category = z.infer<typeof categorySchema>;
export const checkSchema = measurementSchema.extend({
  id: z.string().max(80),
  category: categorySchema,
  source: z.string().max(500),
  observation: z.string().max(1000),
  checkedAt: z.string().datetime(),
});
export type Check = z.infer<typeof checkSchema>;
export const reportSchema = z.object({
  checkedAt: z.string(),
  mode: z.enum(["discovery", "explicit", "mixed"]),
  checks: z.array(checkSchema).max(40),
  coverage: z.literal(
    "Task requirements must be checked separately; passing measurements do not prove readiness.",
  ),
});
export type Report = z.infer<typeof reportSchema>;
export const preflightRpc = defineRpc({
  name: "preflight.run",
  input: z.object({ workspaceId: z.string().min(1) }),
  output: reportSchema,
});
