import { parseNextPrompts, type NextPromptsV1 } from "./next-prompts";

// `whys[i]` is the optional reason shown under `prompts[i]`; it is never sent.
// `threads[i]` marks a suggestion unrelated to the current goal, started only in a new thread.
// `suggestions[i]` marks a prompt the agent recommends; it is display only, never sent or selected on.
// The array exists only when at least one prompt is marked.
export type PromptBlock = {
  block: string;
  prompts: string[];
  whys: string[];
  threads: boolean[];
  suggestions?: boolean[];
  declaration?: NextPromptsV1;
};

export type GitAction = {
  kind: "commit" | "commit-push" | "push";
  label: "Commit" | "Commit & Push" | "Push";
  prompt: string;
};

// Recognize explicit actions conservatively; mentions and unclear wording keep normal Send.
export function gitAction(text: string): GitAction | null {
  const normalized = text
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .replace(/[đĐ]/g, "d")
    .toLowerCase();
  const skill = /^\s*[/$]commit(?=\s|$)/i.test(text);
  const clauses = normalized
    .replace(/^\s*[/$]commit(?=\s|$)/, "commit")
    .split(/[!?;\n]+|\.(?=\s|$)|\b(?:then|roi|sau do)\b/);
  const commandText = (part: string) =>
    part
      .trim()
      .replace(/^(?:(?:please|hay|vui long|chi can|chi|but|nhung)\s+)+/, "")
      .replace(/^git\s+/, "")
      .replace(/^(?:create(?: a)?|tao(?: mot)?)\s+(?=commit\b)/, "");
  const labelList =
    /^(?:edit|send|commit|push)(?:\s*(?:and|va|&|\+)\s*(?:edit|send|commit|push))*(?:\s+(?:buttons?|controls?|labels?))?$/;
  // Require actual label names; mentioning button tests/fixes must not hide a later Git command.
  const commands = clauses.flatMap((clause) => {
    const labels =
      /\b(?:buttons?|controls?|labels?|nut)(?:\s+(?:named|called|labelled|labeled|co nhan))?(?:\s*:\s*|\s+)(?:edit|send|commit|push)\b|\b(?:edit|send)(?=\s*(?:,|and\b|va\b|[&+]|$))/.test(
        clause,
      );
    return clause
      .split(",")
      .filter((part) => !labels || !labelList.test(part.trim()))
      .map((part) => ({ text: commandText(part), labels }));
  });
  // Commas continue a prohibition's list; an explicit transition starts a new instruction.
  const denialText = normalized
    .replace(/,[ \t]*(?:then|roi|sau do|but|nhung)\b/g, ". ")
    // An unrelated-edit restriction can precede a targeted Git command, not a denial list.
    .replace(
      /(^|[.!?;\n,])[ \t]*((?:without\s+(?:changing|editing|modifying)|(?:do not|don['’]t|never)\s+(?:change|edit|modify)|(?:khong|dung)\s+(?:sua|chinh sua|thay doi))\s+(?:unrelated\s+(?:files|changes|work)|other\s+(?:files|changes)|wip\s+khac|(?:cac\s+)?(?:file|thay doi)\s+(?:khac|khong lien quan))),([^.!?;\n]*)/g,
      (whole, boundary: string, restriction: string, rest: string) => {
        const coordinated =
          /\b(?:and|or|va|hoac|hay)\s+(?:git\s+)?(?:commit|push)\b|,\s*(?:and|or|va|hoac|hay)\b/.test(
            rest,
          );
        if (!restriction.startsWith("without ") && coordinated) return whole;
        const targeted = /^(?:git\s+)?(?:commit|push)\s+(?!(?:and|or|va|hoac|hay)\b|[&+])\S/.test(
          rest.trim(),
        );
        return restriction.startsWith("without ") || targeted
          ? `${boundary}${restriction}. ${rest}`
          : whole;
      },
    );
  const denied = (verb: string, scopeException?: RegExp) =>
    [
      ...denialText.matchAll(
        new RegExp(
          `\\b(?:do not|don['’]t|never|not|no|without|avoid|skip|khong|chua|dung|cam)(?:[ \\t,]+\\w+)*?[ \\t,]+${verb}\\b`,
          "g",
        ),
      ),
    ].some((match) => !scopeException?.test(denialText.slice(match.index! + match[0].length)));
  const isCommit = (command: string) =>
    /^commit(?=[:\s]|$)(?!\s+(?:messages?|buttons?|hash|sha|history|status|diff|log|is|was|has|da|already|done|completed)\b)/.test(
      command,
    );
  const isPush = (command: string) =>
    /^push(?=[:\s]|$)(?!\s+(?:notifications?|buttons?|events?|messages?|behavior|is|was|has|da|already|done|completed)\b)/.test(
      command,
    );
  // Excluding unrelated work narrows a positive commit request; it does not cancel it.
  const unrelatedScope =
    /^\s+(?:unrelated\b|other\s+(?:files|changes)\b|wip\s+khac\b|(?:cac\s+)?(?:file|thay doi)\s+(?:khac|khong lien quan)\b)/;
  const commit =
    !denied("commit(?:ting|s)?", unrelatedScope) &&
    (skill || commands.some(({ text }) => isCommit(text)));
  const noPush = /--no-push\b/.test(normalized) || denied("push(?:ing)?");
  const push =
    !noPush &&
    (commands.some(({ text }) => isPush(text)) ||
      (commit &&
        commands.some(
          ({ text, labels }) =>
            isCommit(text) &&
            text
              .split(/\b(?:and|va)\b|[&+]/)
              .slice(1)
              .some(
                (part) => isPush(commandText(part)) && (!labels || !labelList.test(part.trim())),
              ),
        )));
  if (!commit) return push ? { kind: "push", label: "Push", prompt: text } : null;

  const command = push ? "/commit" : "/commit --no-push";
  const prompt = skill
    ? text.replace(
        /^\s*[/$]commit(?=\s|$)/i,
        push || /--no-push\b/i.test(text) ? "/commit" : command,
      )
    : `${command}\n${text}`;
  return {
    kind: push ? "commit-push" : "commit",
    label: push ? "Commit & Push" : "Commit",
    prompt: `${prompt}\nCommit only the described changes. Preserve unrelated work.`,
  };
}

// Accept only top-level fenced suggestions in an explicit next-step section.
export function parsePrompts(markdown: string): PromptBlock[] {
  const result: PromptBlock[] = [];
  let section = false;
  let fence: {
    char: string;
    length: number;
    eligible: boolean;
    structured: boolean;
    lines: string[];
  } | null = null;
  for (const line of markdown.replace(/\r\n/g, "\n").split("\n")) {
    if (fence) {
      const closing = line.match(/^ {0,3}(`{3,}|~{3,})\s*$/);
      if (closing && closing[1][0] === fence.char && closing[1].length >= fence.length) {
        if (fence.eligible) {
          const block = fence.lines.join("\n").replace(/\n+$/, "");
          if (fence.structured) {
            const declaration = parseNextPrompts(block);
            if (declaration) {
              const suggestions = declaration.prompts.map((p) => p.suggestion === true);
              result.push({
                block,
                prompts: declaration.prompts.map((p) => p.prompt),
                whys: declaration.prompts.map((p) => p.why ?? ""),
                threads: declaration.prompts.map((p) => p.thread === "new"),
                ...(suggestions.some(Boolean) ? { suggestions } : {}),
                declaration,
              });
            }
            fence = null;
            continue;
          }
          const prompts: string[] = [];
          const whys: string[] = [];
          const threads: boolean[] = [];
          const suggestions: boolean[] = [];
          // The first `suggestion:` line of a prompt decides; later ones are dropped, not sent.
          const flagged: boolean[] = [];
          let current: string[] | null = null;
          let valid = true;
          for (const bodyLine of block.split("\n")) {
            if (/^prompt:/i.test(bodyLine)) {
              if (current) prompts.push(current.join("\n").replace(/\n+$/, ""));
              current = [bodyLine.replace(/^prompt:[ \t]?/i, "")];
              whys.push("");
              threads.push(false);
              suggestions.push(false);
              flagged.push(false);
            } else if (current && /^why:/i.test(bodyLine) && !whys[whys.length - 1])
              whys[whys.length - 1] = bodyLine.replace(/^why:/i, "").trim();
            else if (
              current &&
              /^thread:\s*new\s*$/i.test(bodyLine) &&
              !threads[threads.length - 1]
            )
              threads[threads.length - 1] = true;
            else if (current && /^suggestion:[ \t]*(?:true|false)[ \t]*$/i.test(bodyLine)) {
              const last = flagged.length - 1;
              if (!flagged[last]) suggestions[last] = /true/i.test(bodyLine);
              flagged[last] = true;
            } else if (current) current.push(bodyLine);
            else if (bodyLine.trim()) valid = false;
          }
          if (current) prompts.push(current.join("\n").replace(/\n+$/, ""));
          if (valid && prompts.length && prompts.every((p) => p.trim() && p.length <= 16000))
            result.push({
              block,
              prompts,
              whys,
              threads,
              ...(suggestions.some(Boolean) ? { suggestions } : {}),
            });
        }
        fence = null;
        continue;
      }
      fence.lines.push(line);
      continue;
    }
    const heading = line.match(/^ {0,3}#{1,6}\s+(.+?)\s*#*\s*$/);
    if (heading) section = /^(?:what(?:['’]s)? next|next steps)$/i.test(heading[1].trim());
    const opening = line.match(/^ {0,3}(`{3,}|~{3,})([^`]*)$/);
    if (opening)
      fence = {
        char: opening[1][0],
        length: opening[1].length,
        eligible: section && /^(?:text|txt|plaintext|next-prompts)?\s*$/i.test(opening[2].trim()),
        structured: /^next-prompts$/i.test(opening[2].trim()),
        lines: [],
      };
  }
  return result;
}

// Several prompts sent or edited together become one numbered message.
export function joinPrompts(prompts: string[]): string {
  if (prompts.length === 1) return prompts[0];
  return prompts.map((p, i) => `${i + 1}. ${p.replace(/\n/g, "\n   ")}`).join("\n");
}
