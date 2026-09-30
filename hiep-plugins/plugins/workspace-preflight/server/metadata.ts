import { constants } from "node:fs";
import { lstat, open, realpath } from "node:fs/promises";
import path from "node:path";

export function inside(root: string, target: string) {
  const relative = path.relative(root, target);
  return relative !== ".." && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
}

// No directory walks or executable config loaders. Broken links are unsafe, not absent.
export async function metadata(root: string, name: string, limit = 65536): Promise<string | null> {
  const candidate = path.join(root, name);
  const parent = path.dirname(candidate);
  if (parent !== root) {
    try {
      await lstat(parent);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return null;
      throw error;
    }
    if (!inside(root, await realpath(parent))) throw new Error("OUTSIDE_WORKSPACE");
  }
  try {
    await lstat(candidate);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return null;
    throw error;
  }
  const target = await realpath(candidate);
  if (!inside(root, target)) throw new Error("OUTSIDE_WORKSPACE");
  const handle = await open(
    target,
    constants.O_RDONLY | constants.O_NONBLOCK | constants.O_NOFOLLOW,
  );
  try {
    const info = await handle.stat();
    if (!info.isFile() || info.size > limit) throw new Error("SIZE_OR_TYPE");
    const buffer = Buffer.alloc(limit + 1);
    const { bytesRead } = await handle.read(buffer, 0, buffer.length, 0);
    if (bytesRead > limit) throw new Error("SIZE_OR_TYPE");
    return buffer.subarray(0, bytesRead).toString("utf8");
  } finally {
    await handle.close();
  }
}

export async function present(root: string, name: string): Promise<boolean> {
  try {
    await lstat(path.join(root, name));
    const target = await realpath(path.join(root, name));
    if (!inside(root, target)) throw new Error("OUTSIDE_WORKSPACE");
    if (!(await lstat(target)).isFile()) throw new Error("UNSUPPORTED_TYPE");
    return true;
  } catch (error) {
    // A dangling symlink is still an unsupported artifact.
    if ((error as NodeJS.ErrnoException).code === "ENOENT") {
      try {
        await lstat(path.join(root, name));
      } catch (missing) {
        if ((missing as NodeJS.ErrnoException).code === "ENOENT") return false;
      }
    }
    throw error;
  }
}
