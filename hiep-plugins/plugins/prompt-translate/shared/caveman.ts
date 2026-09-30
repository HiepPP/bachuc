// Only remove the block owned by this plugin; explicit user commands are preserved.
export function stripCavemanMode(prompt: string): string {
  return prompt.replace(
    /^[/$]caveman[^\n]*\n\n\[prompt-translate:response-mode\]\n[\s\S]*?\n\[\/prompt-translate:response-mode\]\n\n/,
    "",
  );
}

export function preserveCavemanCommand(body: string, draft: string) {
  const explicit =
    /(?:^|[\s;])([/$]caveman(?::caveman)?(?:[ \t]+(?:wenyan-(?:lite|full|ultra)|wenyan|lite|full|ultra|off|stop|disable))?)(?=\s|$)/i.exec(
      draft,
    )?.[1];
  return explicit && !body.includes(explicit) ? `${explicit}\n\n${body}` : body;
}
