import { describe, expect, it } from "vitest";
import { cssVariablesStyle, PREVIEW_SANDBOX, withPreviewCsp } from "@/file-pane/html-preview-csp";

describe("PREVIEW_SANDBOX", () => {
  it("grants scripts only, so frames keep an opaque origin and cannot navigate the app", () => {
    expect(PREVIEW_SANDBOX.split(/\s+/)).toEqual(["allow-scripts"]);
  });
});

describe("withPreviewCsp options", () => {
  it("keeps the offline policy when the network is not requested", () => {
    expect(withPreviewCsp("<p>x</p>", { network: false })).toBe(withPreviewCsp("<p>x</p>"));
    expect(withPreviewCsp("<p>x</p>")).toContain("connect-src 'none'");
  });

  it("adds the height reporter after the policy only when asked", () => {
    const source = "<p>Reply</p>";
    const reporting = withPreviewCsp(source, { reportHeight: true });

    expect(withPreviewCsp(source)).not.toContain("paseoHtmlFrameHeight");
    expect(reporting).toContain("paseoHtmlFrameHeight");
    expect(reporting.indexOf("Content-Security-Policy")).toBeLessThan(
      reporting.indexOf("paseoHtmlFrameHeight"),
    );
    expect(reporting.endsWith(source)).toBe(true);
  });

  it("allows https scripts, styles, fonts, images, and fetch for a networked frame", () => {
    const document = withPreviewCsp("<p>x</p>", { network: true });

    expect(document).toMatch(/script-src[^;]*https:/);
    expect(document).toMatch(/style-src[^;]*https:/);
    expect(document).toMatch(/font-src[^;]*https:/);
    expect(document).toMatch(/img-src[^;]*https:/);
    expect(document).toContain("connect-src https:");
    expect(document).not.toMatch(/http:/);
    expect(document).toContain("form-action 'none'");
    expect(document).toContain("base-uri 'none'");
    expect(document).toContain("frame-src 'none'");
    expect(document).toContain("object-src 'none'");
  });

  it("sets theme variables after the policy and before the document", () => {
    const source = "<h1>Reply</h1>";
    const document = withPreviewCsp(source, {
      themeStyle: cssVariablesStyle({
        "paseo-foreground": "#111",
        "paseo-font-ui": '"Inter", sans-serif',
      }),
    });

    expect(document).toContain(
      '<style>:root { --paseo-foreground: #111; --paseo-font-ui: "Inter", sans-serif; }</style>',
    );
    expect(document.indexOf("Content-Security-Policy")).toBeLessThan(document.indexOf(":root"));
    expect(document.endsWith(source)).toBe(true);
  });

  it("drops a variable whose name or value could break out of the style", () => {
    expect(
      cssVariablesStyle({
        "paseo-ok": "red",
        "bad name": "blue",
        "paseo-escape": "red;} body { display: none",
        "paseo-close": "</style><script>x()</script>",
        "paseo-comment": "red /* rest",
        "paseo-quote": '"Fira Code',
        "paseo-paren": "rgba(0,0,0",
        "paseo-font": "'SF Mono', ui-monospace",
      }),
    ).toBe("<style>:root { --paseo-ok: red; --paseo-font: 'SF Mono', ui-monospace; }</style>");
    expect(cssVariablesStyle({})).toBe("");
  });
});

describe("withPreviewCsp", () => {
  it("places the policy before the complete source document", () => {
    const source =
      " <!-- untrusted --!><!doctype html \"'><script>location='https://example.com'</script>";
    const output = withPreviewCsp(source);

    expect(output).toMatch(/^<!doctype html><meta http-equiv="Content-Security-Policy"/);
    expect(output.endsWith(source)).toBe(true);
    expect(output.indexOf("Content-Security-Policy")).toBeLessThan(output.indexOf("<script>"));
  });

  it("keeps the original document intact", () => {
    const source = "<!doctype html><html><head></head><body><h1>Visual plan</h1></body></html>";

    expect(withPreviewCsp(source)).toContain(source);
  });

  it("drops a leading BOM before appending the source", () => {
    const output = withPreviewCsp("﻿<!doctype html><h1>Plan</h1>");

    expect(output.includes("﻿")).toBe(false);
    expect(output).toContain("<!doctype html><h1>Plan</h1>");
  });

  it("refuses remote resources while allowing inline scripts and styles", () => {
    const document = withPreviewCsp("<h1>Plan</h1>");

    expect(document).toContain("default-src 'none'");
    expect(document).toContain("connect-src 'none'");
    expect(document).toContain("form-action 'none'");
    expect(document).toContain("base-uri 'none'");
    expect(document).toContain("frame-src 'none'");
    expect(document).toContain("object-src 'none'");
    expect(document).toContain("script-src 'unsafe-inline'");
    expect(document).toContain("style-src 'unsafe-inline'");
    expect(document).not.toMatch(/script-src[^;]*https:/);
    expect(document).not.toMatch(/img-src[^;]*https:/);
    expect(document).not.toMatch(/connect-src[^;]*https:/);
  });

  it("handles pathological source without parsing it", () => {
    const source = `${"<!--".repeat(10_000)}${'"'.repeat(10_000)}<html></html>`;

    expect(withPreviewCsp(source).endsWith(source)).toBe(true);
  });
});
