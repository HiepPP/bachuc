import assert from "node:assert/strict";
import test from "node:test";
import { chipColors, projectInitial } from "../client/project-header";

const theme = { surface0: "#fff", surface2: "#eee", border: "#ccc", foregroundMuted: "#666" };

test("the mark shows the first character, uppercased, of the trimmed name", () => {
  assert.equal(projectInitial(" hiep-paseo-plugin "), "H");
  assert.equal(projectInitial("élan"), "É");
  assert.equal(projectInitial("  "), "?");
});

test("a project with a Board hue is tinted with it", () => {
  assert.deepEqual(chipColors(252, theme), {
    background: "hsla(252, 60%, 72%, 0.2)",
    border: "hsla(252, 60%, 72%, 0.8)",
    markBackground: "hsl(252, 42%, 58%)",
    markText: "#fff",
  });
});

test("a project without a hue gets the neutral theme tint", () => {
  assert.deepEqual(chipColors(undefined, theme), {
    background: "#eee",
    border: "#ccc",
    markBackground: "#666",
    markText: "#fff",
  });
});
