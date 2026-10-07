import { describe, expect, it } from "vitest";
import { cssVariablesStyle } from "@/file-pane/html-preview-csp";
import { darkTheme, lightTheme } from "@/styles/theme";
import { htmlFrameCssVariables } from "./html-frame-theme";

describe("htmlFrameCssVariables", () => {
  it.each([
    ["light", lightTheme],
    ["dark", darkTheme],
  ])(
    "keeps every %s theme variable, font stacks included, through the style filter",
    (_, theme) => {
      const variables = htmlFrameCssVariables(theme);
      const style = cssVariablesStyle(variables);

      for (const [name, value] of Object.entries(variables)) {
        expect(style).toContain(`--${name}: ${value};`);
      }
      expect(Object.keys(variables)).toHaveLength(11);
    },
  );
});
