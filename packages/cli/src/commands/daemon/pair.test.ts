import { mkdtemp, rm, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { test, expect } from "vitest";
import { resolveLocalPairingOffer } from "./pair.js";

test("offline pairing needs relay consent, a relay endpoint, and an app base URL", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "paseo-offline-pair-"));
  const home = path.join(root, "home");
  try {
    expect(await resolveLocalPairingOffer({ paseoHome: home })).toMatchObject({
      relayEnabled: false,
      url: null,
    });
    expect(existsSync(home)).toBe(false);
    expect(await resolveLocalPairingOffer({ paseoHome: home, enableRelay: true })).toMatchObject({
      relayEnabled: true,
      url: null,
    });
    const configPath = path.join(home, "config.json");
    const config = JSON.parse(await readFile(configPath, "utf8"));
    expect(config.daemon.relay.enabled).toBe(true);

    config.daemon.relay.endpoint = "relay.example.com:443";
    await writeFile(configPath, JSON.stringify(config));
    expect(await resolveLocalPairingOffer({ paseoHome: home })).toMatchObject({
      relayEnabled: true,
      url: null,
    });

    config.app = { baseUrl: "https://app.example.com" };
    await writeFile(configPath, JSON.stringify(config));
    const offer = await resolveLocalPairingOffer({ paseoHome: home });
    expect(offer.relayEnabled).toBe(true);
    expect(offer.url).toContain("https://app.example.com/#offer=");
    expect(existsSync(path.join(home, "server-id"))).toBe(true);
    expect(existsSync(path.join(home, "daemon-keypair.json"))).toBe(true);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
