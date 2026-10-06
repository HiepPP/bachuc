import { mkdir, readFile, rename, unlink, writeFile } from "node:fs/promises";
import path from "node:path";
import { PAGE_ID } from "../shared/row";

export const MAX_HTML_BYTES = 1024 * 1024;

/** Why the HTML cannot be published, or null when it can. */
export function htmlProblem(html: string): string | null {
  if (!html.trim()) return "The page is empty.";
  const bytes = Buffer.byteLength(html, "utf8");
  if (bytes > MAX_HTML_BYTES) {
    return `The page is ${bytes} bytes; the limit is ${MAX_HTML_BYTES} bytes (1 MiB). Inline less data or load large libraries from an https CDN.`;
  }
  return null;
}

export interface HtmlStore {
  save(id: string, html: string): Promise<void>;
  /** The page, or null for an unknown or malformed id. */
  read(id: string): Promise<string | null>;
  remove(id: string): Promise<void>;
}

export function createHtmlStore(dir: string): HtmlStore {
  // The id arrives from the app over RPC, so it must never reach the file system unchecked.
  function fileOf(id: string): string | null {
    return PAGE_ID.test(id) ? path.join(dir, `${id}.html`) : null;
  }

  return {
    async save(id, html) {
      const file = fileOf(id);
      if (!file) throw new Error(`Invalid page id: ${id}`);
      const problem = htmlProblem(html);
      if (problem) throw new Error(problem);
      await mkdir(dir, { recursive: true });
      const temp = `${file}.${process.pid}.tmp`;
      await writeFile(temp, html, "utf8");
      await rename(temp, file);
    },
    async read(id) {
      const file = fileOf(id);
      if (!file) return null;
      try {
        return await readFile(file, "utf8");
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code === "ENOENT") return null;
        throw error;
      }
    },
    async remove(id) {
      const file = fileOf(id);
      if (!file) return;
      await unlink(file).catch((error: NodeJS.ErrnoException) => {
        if (error.code !== "ENOENT") throw error;
      });
    },
  };
}
