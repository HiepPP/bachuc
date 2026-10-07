import { useRpc, type PluginTimelineItemProps } from "@getpaseo/plugin/client";
import { HtmlFrame } from "@getpaseo/plugin/client/ui";
import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { Text, View } from "react-native";
import { getHtmlReplyRpc, type HtmlReplyRow } from "../shared/row";

type Theme = PluginTimelineItemProps["theme"];

function useStyles(theme: Theme) {
  return useMemo(
    () => ({
      card: { gap: 8 },
      title: { color: theme.colors.foreground, fontWeight: "600" as const },
      note: { color: theme.colors.foregroundMuted, fontSize: 13 },
    }),
    [theme],
  );
}

function pageNote(status: "pending" | "error" | "success", missing: boolean): string | null {
  if (status === "pending") return "Loading page…";
  if (status === "error") return "Could not load this page.";
  return missing ? "Page not found. Its file may have been removed." : null;
}

/**
 * The title, then the whole page in the host's sandboxed frame, sized to its content, with
 * network access (ADR-0002).
 */
export function HtmlReplyCard({ item, theme }: PluginTimelineItemProps<HtmlReplyRow>) {
  const styles = useStyles(theme);
  const readPage = useRpc(getHtmlReplyRpc);
  const { id, title } = item.data;
  const page = useQuery({
    queryKey: ["html-reply-page", id],
    queryFn: () => readPage({ id }),
    // A published page never changes.
    staleTime: Infinity,
    retry: false,
  });
  const html = page.data?.html ?? null;
  const note = pageNote(page.status, page.status === "success" && html === null);
  return (
    <View style={styles.card}>
      <Text style={styles.title}>{title}</Text>
      {note || html === null ? (
        <Text style={styles.note}>{note}</Text>
      ) : (
        <HtmlFrame html={html} title={title} height="auto" network />
      )}
    </View>
  );
}
