export const REPLICA_ROW_STORE_SCHEMA_VERSION = 2;

export const REPLICA_SINGLETON_ROW_ID = "singleton";

/** UTF-8 size of a payload. Both backends store this size so their totals match. */
export function replicaPayloadBytes(payload: string): number {
  // Avoids the JS Buffer polyfill, which materialises the whole byte array just to measure it.
  let bytes = 0;
  for (let index = 0; index < payload.length; index += 1) {
    const code = payload.charCodeAt(index);
    if (code < 0x80) bytes += 1;
    else if (code < 0x800) bytes += 2;
    else if (code >= 0xd800 && code <= 0xdbff) {
      bytes += 4;
      index += 1;
    } else bytes += 3;
  }
  return bytes;
}
