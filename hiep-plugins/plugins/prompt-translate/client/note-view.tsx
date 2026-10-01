import type { PluginTimelineItemProps } from "@getpaseo/plugin/client";
import { useEffect, useState } from "react";
import { Text, View } from "react-native";
import { z } from "zod";
import type { Note } from "./notes";

export const noteSchema = z.object({ text: z.string() });

/** `lookup` is bound at registration so every row shares one cache. */
export function createNoteView(
  lookup: (text: string, cacheOnly: boolean) => Promise<Note>,
  activeSince: () => number,
) {
  return function NoteView({
    item,
    timestamp,
    theme,
  }: PluginTimelineItemProps<z.output<typeof noteSchema>>) {
    const [note, setNote] = useState<Note>(null);
    const text = item.data.text;
    const cacheOnly = timestamp.getTime() < activeSince();
    useEffect(() => {
      let active = true;
      void lookup(text, cacheOnly).then((value) => active && setNote(value));
      return () => {
        active = false;
      };
    }, [text, cacheOnly]);
    if (!note) return null;
    return (
      <View style={{ paddingHorizontal: 12, paddingBottom: 6, alignItems: "flex-end" }}>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: 12, lineHeight: 17 }}>
          <Text style={{ fontWeight: "600" }}>{note.label} </Text>
          {note.text}
        </Text>
      </View>
    );
  };
}
