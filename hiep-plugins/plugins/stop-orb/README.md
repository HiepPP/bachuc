# Stop orb

Replaces the composer's red stop button with the `solving` thinking orb from
[thinking-orbs](https://libraries.dev/orbs) while an agent runs. Press the orb to stop the
agent, as before. The tooltip and the interrupt shortcut stay. The orb freezes after you press
it, until the agent stops.

- The orb turns red on hover, in the red of Paseo's stop button, to show that a press stops the agent.
- Web and desktop only. thinking-orbs draws on a DOM canvas, so iOS and Android keep Paseo's red button.
- The orb's sphere is as wide as the composer's other icons (16 px on web). Its theme follows the Paseo surface color.
- No server entry, RPC, settings, or network requests.

## Host requirements

Needs the fork's `addComposerStopButton` client contribution. An older app fails to load the plugin.

## Checks

```sh
npm run format
npm run typecheck
npm run lint
npm test
```
