import type { TokenUsage } from "./usage";

const codes = [
  "JEV_CONFIG_INVALID",
  "JEV_CREDENTIAL_MISSING",
  "JEV_INPUT_INVALID",
  "JEV_TYPESAFE_HTTP",
  "JEV_TYPESAFE_FAILED",
  "JEV_GATEWAY_HTTP",
  "JEV_MODEL_ACCESS_DENIED",
  "JEV_GATEWAY_FAILED",
  "JEV_RESPONSE_INVALID",
  "JEV_WORKER_SPAWN_FAILED",
  "JEV_WORKER_EXIT_FAILED",
  "JEV_WORKER_OUTPUT_INVALID",
  "JEV_WORKER_OUTPUT_LIMIT",
  "JEV_TIMEOUT",
  "JEV_CANCELLED",
] as const;
export type EvaluationCode = (typeof codes)[number];
export type EvaluationDiagnostics = {
  workerCode: EvaluationCode;
  httpStatus?: number;
  processExitCode?: number | null;
};
export function safeEvaluationDiagnostics(value: unknown): EvaluationDiagnostics | undefined {
  if (!value || typeof value !== "object") return;
  const v = value as Record<string, unknown>;
  if (!codes.includes(v.workerCode as EvaluationCode)) return;
  const result: EvaluationDiagnostics = { workerCode: v.workerCode as EvaluationCode };
  if (Number.isInteger(v.httpStatus) && Number(v.httpStatus) >= 100 && Number(v.httpStatus) <= 599)
    result.httpStatus = v.httpStatus as number;
  if (
    v.processExitCode === null ||
    (Number.isInteger(v.processExitCode) &&
      Number(v.processExitCode) >= 0 &&
      Number(v.processExitCode) <= 255)
  )
    result.processExitCode = v.processExitCode as number | null;
  return result;
}
export class EvaluationError extends Error {
  usage?: TokenUsage;
  readonly diagnostics: EvaluationDiagnostics;
  constructor(
    workerCode: EvaluationCode,
    details: { httpStatus?: unknown; processExitCode?: unknown } = {},
  ) {
    super(
      workerCode === "JEV_MODEL_ACCESS_DENIED"
        ? "Jev model access denied by Gateway account tier. Use a Gateway account/key authorized for typesafe-ai/jev; available credits alone do not grant access."
        : `Jev evaluation failed (${workerCode}); no automatic fallback or retry.`,
    );
    this.diagnostics = safeEvaluationDiagnostics({ workerCode, ...details })!;
  }
}

// Recognize the observed Gateway restriction without exposing response bodies or credentials.
export function gatewayEvaluationError(error: unknown): EvaluationError {
  const outer = error && typeof error === "object" ? (error as Record<string, unknown>) : {};
  const status = outer.statusCode;
  let current: unknown = error;
  let restricted = false;
  for (let depth = 0; depth < 3 && current && typeof current === "object"; depth++) {
    const value = current as Record<string, unknown>;
    for (const field of [value.message, value.responseBody]) {
      if (
        typeof field === "string" &&
        /RestrictedModelsError|Free tier users do not have access to this model/i.test(
          field.slice(0, 16384),
        )
      )
        restricted = true;
    }
    current = value.cause;
  }
  return new EvaluationError(
    status === 403 && restricted
      ? "JEV_MODEL_ACCESS_DENIED"
      : typeof status === "number"
        ? "JEV_GATEWAY_HTTP"
        : "JEV_GATEWAY_FAILED",
    { httpStatus: status },
  );
}

// Never return the provider's raw response, request headers, or error message.
export function typesafeEvaluationError(error: unknown): EvaluationError {
  const status =
    error && typeof error === "object" && "statusCode" in error ? error.statusCode : undefined;
  return new EvaluationError(
    typeof status === "number" ? "JEV_TYPESAFE_HTTP" : "JEV_TYPESAFE_FAILED",
    { httpStatus: status },
  );
}
