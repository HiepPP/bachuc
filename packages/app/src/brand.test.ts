import { describe, expect, it } from "vitest";
import { BRAND_NAME, brandText, withBrandName } from "./brand";

describe("brandText", () => {
  it("replaces the whole word Paseo", () => {
    expect(brandText("Welcome to Paseo")).toBe(`Welcome to ${BRAND_NAME}`);
    expect(brandText("Quitting Paseo...")).toBe(`Quitting ${BRAND_NAME}...`);
  });

  it("keeps env vars, lowercase names, and code symbols", () => {
    expect(brandText("assigns a port via $PASEO_PORT")).toBe("assigns a port via $PASEO_PORT");
    expect(brandText("run paseo ls")).toBe("run paseo ls");
    expect(brandText("PaseoLogo")).toBe("PaseoLogo");
  });

  it("keeps placeholders", () => {
    expect(brandText("Paseo {{Paseo}} {{count}}")).toBe(`${BRAND_NAME} {{Paseo}} {{count}}`);
  });
});

describe("withBrandName", () => {
  it("brands nested and plural strings and leaves the input unchanged", () => {
    const en = {
      welcome: { title: "Welcome to Paseo" },
      calls: { one: "called Paseo {{count}} time", other: "called Paseo {{count}} times" },
      hint: "Paseo assigns $PASEO_PORT",
    };
    const resources = { en: { translation: en } };

    expect(withBrandName(resources)).toEqual({
      en: {
        translation: {
          welcome: { title: `Welcome to ${BRAND_NAME}` },
          calls: {
            one: `called ${BRAND_NAME} {{count}} time`,
            other: `called ${BRAND_NAME} {{count}} times`,
          },
          hint: `${BRAND_NAME} assigns $PASEO_PORT`,
        },
      },
    });
    expect(en.welcome.title).toBe("Welcome to Paseo");
  });
});
