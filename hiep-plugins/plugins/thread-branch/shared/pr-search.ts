import {
  defineAttachmentSource,
  defineRpc,
  PluginAttachmentSearchPayloadSchema,
} from "@getpaseo/plugin";
import { z } from "zod";

export const searchPrsRpc = defineRpc({
  name: "thread-branch.search-prs",
  input: z.object({ query: z.string().max(512) }),
  output: PluginAttachmentSearchPayloadSchema,
});

export const prAttachments = defineAttachmentSource({
  id: "github-pr",
  title: "GitHub PR",
  icon: "GitPullRequest",
  pickerTitle: "Attach an open pull request or its failed CI log",
  searchPlaceholder: "Search by repo, #number, title, or branch",
  search: searchPrsRpc,
});
