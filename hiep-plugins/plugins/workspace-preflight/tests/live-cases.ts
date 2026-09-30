import { evaluationInput, type Task } from "../shared/evaluation";
import type { Report } from "../shared/preflight";

export function liveCases() {
  const report = (status: "blocker" | "unknown"): Report => ({
    checkedAt: "2026-09-20T00:00:00.000Z",
    mode: "explicit",
    coverage:
      "Task requirements must be checked separately; passing measurements do not prove readiness.",
    checks: [
      {
        id: "health:0",
        category: "health",
        label: "Synthetic backend",
        source: "synthetic fixture, not a live workspace measurement",
        checkedAt: "2026-09-20T00:00:00.000Z",
        status,
        detail:
          status === "blocker"
            ? "HTTP 503; expected 200."
            : "Backend health has not been measured.",
        observation:
          status === "blocker"
            ? "HTTP 503; expected 200."
            : "Backend health has not been measured.",
        repair: "",
      },
    ],
  });
  const task = (summary: string, login: boolean, unknown = false): Task => ({
    summary,
    requiredChecks: login
      ? [{ id: "health:0", reason: "Real login verification requires available backend." }]
      : [],
    coverageGaps: unknown ? ["Backend health is unknown."] : [],
  });
  return [
    {
      id: "docs",
      input: evaluationInput(
        task(
          "Edit prose in README only. No build, code execution, preview, backend, or login required. Task scope is fully known.",
          false,
        ),
        report("blocker"),
      ),
    },
    {
      id: "login-down",
      input: evaluationInput(
        task(
          "Verify login UI with real authentication. Authorized disposable credentials and all other prerequisites are ready; backend is required.",
          true,
        ),
        report("blocker"),
      ),
    },
    {
      id: "login-unknown",
      input: evaluationInput(
        task(
          "Verify login UI with real authentication. Authorized disposable credentials and all other prerequisites are ready; backend is required.",
          true,
          true,
        ),
        report("unknown"),
      ),
    },
  ];
}
// Acceptance labels are intentionally separate from model inputs. Never retry model answers.
export const expectedChoices = {
  docs: "proceed",
  "login-down": "prepare_environment",
  "login-unknown": "need_more_evidence",
} as const;
