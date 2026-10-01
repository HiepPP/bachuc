import { hasVietnamese } from "../shared/vietnamese";

export type Note = { label: "EN" | "VI gốc"; text: string } | null;

type Lookups = {
  translate(text: string, cacheOnly: boolean): Promise<string | null>;
  original(text: string): Promise<string | null>;
};

/**
 * One lookup per distinct text and cache mode, shared by every render of that message.
 * Vietnamese text gets its English translation; an English rewrite gets its Vietnamese original.
 */
export function createNoteCache(lookups: Lookups) {
  const notes = new Map<string, Promise<Note>>();
  return (text: string, cacheOnly: boolean): Promise<Note> => {
    const key = `${cacheOnly ? "cached" : "live"}\u0000${text}`;
    let note = notes.get(key);
    if (!note) {
      note = (
        hasVietnamese(text)
          ? lookups
              .translate(text, cacheOnly)
              .then((value): Note => (value ? { label: "EN", text: value } : null))
          : lookups
              .original(text)
              .then((value): Note => (value ? { label: "VI gốc", text: value } : null))
      ).catch((): Note => {
        notes.delete(key);
        return null;
      });
      notes.set(key, note);
    }
    return note;
  };
}
