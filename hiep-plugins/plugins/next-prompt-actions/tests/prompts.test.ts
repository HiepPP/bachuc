import { test } from "node:test";
import assert from "node:assert/strict";
import { gitAction, parsePrompts } from "../shared/prompts";

test("explicit Git actions distinguish commit, commit and push, and push only", () => {
  for (const [text, kind] of [
    ["Commit thay đổi. Không push.", "commit"],
    ["Review, then COMMIT.", "commit"],
    ["Hãy commit thay đổi trong plugin.", "commit"],
    ["Please git commit the fix.", "commit"],
    ["Create a commit for the fix.", "commit"],
    ["Tạo commit cho thay đổi.", "commit"],
    ["Commit and push the fix.", "commit-push"],
    ["Commit và push thay đổi.", "commit-push"],
    ["Commit & Push.", "commit-push"],
    ["Commit xong rồi push origin/main.", "commit-push"],
    ["Commit the fix, then push origin/main.", "commit-push"],
    ["Commit and push the fix. Do not commit unrelated files.", "commit-push"],
    ["Kiểm tra diff, commit phần sửa lỗi.", "commit"],
    ["Run tests, commit the fix.", "commit"],
    ["Commit the fix, push origin/main.", "commit-push"],
    ["Commit the button fixes, then push origin/main.", "commit-push"],
    ["Commit the button fixes and push origin/main.", "commit-push"],
    ["Commit the button fixes, git push.", "commit-push"],
    ["Commit the button fixes and git push.", "commit-push"],
    ["Review the buttons, commit the fix.", "commit"],
    ["Run button tests, commit.", "commit"],
    ["Chạy test cho các nút, commit.", "commit"],
    ["Commit the button fixes and push.", "commit-push"],
    ["Do not edit, then commit the fix.", "commit"],
    ["Do not edit, but commit the fix.", "commit"],
    ["Do not commit, then push the existing commit.", "push"],
    ["/commit\nCommit and push the fix.", "commit-push"],
    ["$commit and push the fix.", "commit-push"],
    ["Push commit 5324b19 lên origin/main.", "push"],
    ["Đã commit xong. Hãy push nhánh main.", "push"],
    ["Do not commit. Push the existing commit.", "push"],
  ])
    assert.equal(gitAction(text)?.kind, kind, text);
});

test("mentions, negative requests, and existing commits do not imply a new commit", () => {
  for (const text of [
    "Review commit 5324b19.",
    "Review commit and push behavior.",
    "Kiểm tra commit gần nhất.",
    "Không commit hoặc push.",
    "Do not commit or push.",
    "Don't commit and push.",
    "Never git commit changes.",
    "Avoid committing changes.",
    "Commit message needs review.",
    "Commit button needs review.",
    "Commit đã xong.",
    "Review the commitment.",
    "Review the Edit, Commit and Push buttons.",
    "Review nút Edit, Commit và Push.",
    "Review nút Commit, Push.",
    "Review buttons named Commit, Push.",
    "Review các nút có nhãn Commit, Push.",
    "Review commit abc123. Không sửa code, commit hoặc push.",
    "Review the change. Do not edit, commit or push.",
    "Review the change. Don't edit, commit, or push.",
    "Review the change. Never edit files, commit changes, or push them.",
    "Do not edit unrelated files, commit changes, or push them.",
    "Do not edit unrelated files, commit changes and push them.",
    "Do not edit unrelated files, commit & push.",
    "Không sửa WIP khác, commit hoặc push.",
    "Inspect src/commit.ts and push.ts.",
    "Push notifications need review.",
    "Push đã xong.",
    "Run tests.",
  ])
    assert.equal(gitAction(text), null, text);
});

test("button-list ordering never authorizes push", () => {
  const names = ["Edit", "Send", "Push"];
  for (const first of names)
    for (const second of names.filter((name) => name !== first)) {
      const third = names.find((name) => name !== first && name !== second)!;
      for (const text of [
        `Commit sửa nút ${first}, ${second} và ${third}.`,
        `Commit updates to ${first}, ${second} and ${third}.`,
        `Commit changes to ${first}, ${second}, and ${third} buttons.`,
      ]) {
        assert.equal(gitAction(text)?.kind, "commit", text);
        assert.ok(gitAction(text)?.prompt.startsWith("/commit --no-push\n"), text);
      }
    }
  assert.equal(gitAction("Commit sửa nút Edit và Push.")?.kind, "commit");
});

test("push mentions outside an explicit command do not turn a commit into a push", () => {
  for (const text of [
    "Commit the fix. Review commit and push behavior.",
    "Commit changes to commit and push buttons.",
    "Commit the push notification fix.",
    "Commit sửa nút Edit, Push và Send.",
    "Commit changes to Edit, Push, and Send buttons.",
    "Commit changes to buttons named Commit and Push.",
    "Commit changes to buttons called Commit, Push.",
    "Commit sửa các nút có nhãn Commit, Push.",
    "Commit sửa các nút:Commit, Push.",
  ]) {
    const action = gitAction(text)!;
    assert.equal(action.kind, "commit", text);
    assert.ok(action.prompt.startsWith("/commit --no-push\n"), text);
  }
});

test("excluding unrelated files preserves a positive commit request, not a blanket prohibition", () => {
  for (const text of [
    "Commit plugin changes. Không commit WIP khác.",
    "Không commit WIP khác. Commit plugin changes.",
    "Commit the fix. Do not commit unrelated files.",
    "Commit the fix. Không commit thay đổi không liên quan.",
    "/commit only the fix. Do not commit unrelated files.",
    "Without changing unrelated files, commit the fix.",
    "Do not edit unrelated files, commit the fix.",
    "Không sửa WIP khác, commit bản sửa.",
  ]) {
    assert.equal(gitAction(text)?.kind, "commit", text);
    assert.ok(gitAction(text)?.prompt.startsWith("/commit --no-push"), text);
  }
  for (const text of [
    "Không commit WIP khác.",
    "Do not commit unrelated files.",
    "Commit the fix. Không commit.",
    "Commit the fix. Do not commit any changes.",
    "Commit the fix. Do not commit unrelated files. Do not commit.",
  ])
    assert.equal(gitAction(text), null, text);
});

test("scoped edit restrictions preserve explicit push commands and later no-push constraints", () => {
  for (const text of [
    "Without changing unrelated files, commit and push the fix.",
    "Không sửa WIP khác, commit bản sửa rồi push.",
    "Commit changes to buttons named Commit and Push, then push.",
  ])
    assert.equal(gitAction(text)?.kind, "commit-push", text);
  assert.equal(
    gitAction("Without changing unrelated files, push the existing commit.")?.kind,
    "push",
  );
  assert.equal(
    gitAction("Without changing unrelated files, commit the fix. Do not push.")?.kind,
    "commit",
  );
});

test("no-push constraints override positive push wording and skill defaults", () => {
  for (const suffix of [
    "Không push.",
    "Chưa push.",
    "Đừng push.",
    "Do not push.",
    "Don't push.",
    "Never push.",
    "Without pushing.",
    "--no-push",
    "Do not commit unrelated files or push.",
    "Do not commit unrelated files and push.",
    "Không commit WIP khác hay push.",
    "Không commit WIP khác hoặc push.",
    "Không commit WIP khác và push.",
    "Không được tự ý thực hiện push.",
    "Do not edit, push.",
  ]) {
    const text = `Commit and push the fix. ${suffix}`;
    const action = gitAction(text)!;
    assert.equal(action.label, "Commit", text);
    assert.ok(action.prompt.startsWith("/commit --no-push\n"), text);
    assert.ok(action.prompt.includes(text), "the original scope and constraints remain intact");
  }
});

test("skill commands are normalized once; push only never invokes the commit skill", () => {
  for (const text of [
    "/commit only the fix",
    "$commit only the fix",
    "/commit --no-push only the fix",
  ]) {
    const action = gitAction(text)!;
    assert.ok(action.prompt.startsWith("/commit --no-push only the fix\n"));
    assert.equal(action.prompt.match(/--no-push/g)?.length, 1);
    assert.equal(action.prompt.match(/\/commit/g)?.length, 1);
    assert.match(action.prompt, /Preserve unrelated work\.$/);
  }
  const push = "Push commit 5324b19 lên origin/main. Giữ nguyên WIP.";
  assert.deepEqual(gitAction(push), { kind: "push", label: "Push", prompt: push });
});

test("headings, multiline Vietnamese, multiple suggestions, and marker removal", () => {
  assert.deepEqual(
    parsePrompts(
      "## What Next\n```text\nprompt: Kiểm tra UI.\nGiữ nguyên WIP.\nprompt: Báo kết quả.\n```",
    ),
    [
      {
        block: "prompt: Kiểm tra UI.\nGiữ nguyên WIP.\nprompt: Báo kết quả.",
        prompts: ["Kiểm tra UI.\nGiữ nguyên WIP.", "Báo kết quả."],
        whys: ["", ""],
        threads: [false, false],
      },
    ],
  );
  assert.equal(parsePrompts("## Next Steps\n~~~\nprompt: Test.\n~~~").length, 1);
});
test("a why: line under a prompt is its reason, not prompt text", () => {
  const [parsed] = parsePrompts(
    "## What Next\n```\nprompt: Commit.\nwhy: Locks in the fix.\nprompt: Push.\n```",
  );
  assert.deepEqual(parsed.prompts, ["Commit.", "Push."]);
  assert.deepEqual(parsed.whys, ["Locks in the fix.", ""]);
  assert.deepEqual(parsePrompts("## What Next\n```\nwhy: Orphan.\nprompt: Test.\n```"), []);
});
test("a thread: new line marks only its own prompt as unrelated work", () => {
  const [parsed] = parsePrompts(
    "## What Next\n```\nprompt: Commit and push the fix.\nprompt: Audit the logs.\nwhy: Separate issue.\nthread: new\n```",
  );
  assert.deepEqual(parsed.prompts, ["Commit and push the fix.", "Audit the logs."]);
  assert.deepEqual(parsed.whys, ["", "Separate issue."]);
  assert.deepEqual(parsed.threads, [false, true]);
});
test("a suggestion: line marks only its own prompt and is never prompt text", () => {
  const [parsed] = parsePrompts(
    [
      "## What Next",
      "```text",
      "prompt: Verify the layout.",
      "suggestion: true",
      "prompt: Review spacing.",
      "Suggestion: FALSE",
      "prompt: Audit the logs.",
      "why: Separate issue.",
      "thread: new",
      "SUGGESTION: True",
      "prompt: Check docs.",
      "suggestion: false",
      "suggestion: true",
      "```",
    ].join("\n"),
  );
  assert.deepEqual(parsed.prompts, [
    "Verify the layout.",
    "Review spacing.",
    "Audit the logs.",
    "Check docs.",
  ]);
  assert.deepEqual(parsed.suggestions, [true, false, true, false], "first occurrence wins");
  assert.deepEqual(parsed.whys, ["", "", "Separate issue.", ""]);
  assert.deepEqual(parsed.threads, [false, false, true, false]);
});
test("an unrecognised suggestion: value is not a flag and the line stays prompt text", () => {
  const [parsed] = parsePrompts(
    "## What Next\n```\nprompt: Verify.\nsuggestion: maybe\nprompt: Review.\nsuggestion: true\n```",
  );
  assert.deepEqual(parsed.prompts, ["Verify.\nsuggestion: maybe", "Review."]);
  assert.deepEqual(parsed.suggestions, [false, true]);
  assert.deepEqual(parsePrompts("## What Next\n```\nsuggestion: true\nprompt: Test.\n```"), []);
});
test("ignore incomplete streams, ordinary code, quotes, empty entries, and other sections", () => {
  for (const text of [
    "```\nprompt: Test\n```",
    "## Next Steps\n```js\nprompt: test\n```",
    "> ## Next Steps\n> ```\n> prompt: Test\n> ```",
    "## Next Steps\n```\nprompt: Test",
    "## Next Steps\n```\nprompt: \n```",
    "## Next Steps\n## Example\n```\nprompt: Test\n```",
    "## Next Steps\n```\nexample\nprompt: Test\n```",
  ])
    assert.deepEqual(parsePrompts(text), []);
});
test("headings inside a code fence cannot enable prompt parsing", () => {
  assert.deepEqual(parsePrompts("````\n## Next Steps\n```\nprompt: Test\n```\n````"), []);
});
