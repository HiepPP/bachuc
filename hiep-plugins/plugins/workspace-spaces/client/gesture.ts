// A quiet gap ends a trackpad gesture; momentum must not skip multiple Spaces.
export function createWheelGesture() {
  let last = -Infinity,
    x = 0,
    y = 0,
    fired = false,
    firedAt = -Infinity,
    previousMagnitude = 0,
    tailMinimum = Infinity,
    rising = 0;
  return (dx: number, dy: number, time: number) => {
    const magnitude = Math.abs(dx);
    const reversed = fired && dx * x < 0 && magnitude >= 2;
    if (fired && time - firedAt > 80) {
      tailMinimum = Math.min(tailMinimum, magnitude);
      rising = magnitude > previousMagnitude ? rising + 1 : 0;
    }
    // Recognize a new stroke that accelerates through the previous momentum tail.
    const freshPush =
      fired &&
      time - firedAt > 160 &&
      magnitude >= Math.max(4, tailMinimum * 1.6) &&
      (rising >= 2 || magnitude >= Math.max(8, previousMagnitude * 2));
    if (time - last > 120 || reversed || freshPush) {
      x = 0;
      y = 0;
      fired = false;
      tailMinimum = Infinity;
      rising = 0;
    }
    last = time;
    previousMagnitude = magnitude;
    x += dx;
    y += Math.abs(dy);
    if (fired || Math.abs(x) < 28 || Math.abs(x) < y * 1.5) return 0;
    fired = true;
    firedAt = time;
    tailMinimum = Infinity;
    return x > 0 ? 1 : -1;
  };
}
export function touchDirection(dx: number, dy: number) {
  return Math.abs(dx) >= 70 && Math.abs(dx) > Math.abs(dy) * 1.5 ? (dx < 0 ? 1 : -1) : 0;
}
