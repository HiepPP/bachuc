import React, { createElement } from "react";
import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { HtmlFrame } from "@/plugins/react-native/html-frame";

// App sources compile against the classic JSX runtime, which expects React on the global.
beforeEach(() => vi.stubGlobal("React", React));

interface FrameReport {
  ran: boolean;
  fetch: "ok" | "failed";
  violations: string[];
  storage: "open" | "blocked";
}

// The page reports back through postMessage, the one channel an opaque-origin frame keeps.
// Port 9 refuses at once, so a fetch the policy allows fails on the network instead of on CSP.
const PROBE = `<script>
  const violations = [];
  document.addEventListener("securitypolicyviolation", (event) => {
    violations.push(event.effectiveDirective + " " + event.blockedURI);
  });
  let storage = "open";
  try { void window.localStorage.length; void document.cookie; } catch { storage = "blocked"; }
  fetch("https://127.0.0.1:9/").then(() => "ok", () => "failed").then((fetchResult) => {
    setTimeout(() => parent.postMessage({ ran: true, fetch: fetchResult, violations, storage }, "*"), 100);
  });
</script>`;

let root: Root | null = null;

afterEach(() => {
  root?.unmount();
  root = null;
  document.body.replaceChildren();
});

// Renders the real plugin component, so the test covers the sandbox attribute the shared view
// sets and the `network` prop reaching the policy.
function renderFrame(network: boolean): { frame: HTMLIFrameElement; report: Promise<FrameReport> } {
  const container = document.createElement("div");
  document.body.append(container);
  let resolveReport: (report: FrameReport) => void = () => undefined;
  const report = new Promise<FrameReport>((resolve, reject) => {
    resolveReport = resolve;
    setTimeout(() => reject(new Error("The frame never reported")), 5000);
  });
  window.addEventListener("message", function onMessage(event) {
    if (event.source !== container.querySelector("iframe")?.contentWindow) return;
    window.removeEventListener("message", onMessage);
    resolveReport(event.data as FrameReport);
  });
  root = createRoot(container);
  flushSync(() => root?.render(createElement(HtmlFrame, { html: PROBE, network, height: 120 })));
  const frame = container.querySelector("iframe");
  if (!frame) throw new Error("HtmlFrame rendered no iframe");
  return { frame, report };
}

it("renders a scripts-only sandbox and refuses an offline fetch through connect-src", async () => {
  const { frame, report } = renderFrame(false);

  expect(frame.getAttribute("sandbox")).toBe("allow-scripts");
  const result = await report;
  expect(result).toMatchObject({ ran: true, fetch: "failed", storage: "blocked" });
  expect(result.violations).toEqual([expect.stringMatching(/^connect-src .*127\.0\.0\.1:9/)]);
});

it("lets a networked frame fetch over https while storage stays blocked", async () => {
  const { frame, report } = renderFrame(true);

  expect(frame.getAttribute("sandbox")).toBe("allow-scripts");
  const result = await report;
  expect(result).toMatchObject({ ran: true, violations: [], storage: "blocked" });
});

function renderAutoFrame(html: string): () => number {
  const container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  flushSync(() => root?.render(createElement(HtmlFrame, { html, height: "auto" })));
  const frame = container.querySelector("iframe");
  if (!frame?.parentElement) throw new Error("HtmlFrame rendered no iframe");
  const body = frame.parentElement;
  return () => body.getBoundingClientRect().height;
}

it("grows an auto-height frame to show the whole page without inner scroll", async () => {
  const height = renderAutoFrame(
    '<body style="margin:0;padding:10px"><div style="height:900px"></div></body>',
  );

  await expect.poll(height, { timeout: 3000 }).toBe(920);
});

it("stops growing a page sized to the frame, such as 100vh", async () => {
  const height = renderAutoFrame(
    '<body style="margin:8px"><div style="height:100vh"></div></body>',
  );

  await new Promise((resolve) => setTimeout(resolve, 1000));
  expect(height()).toBeLessThan(400);
});
