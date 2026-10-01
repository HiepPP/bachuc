// Turns trackpad wheel deltas into at most one swipe per gesture. A quiet gap ends a gesture,
// and the momentum tail must not fire again unless a new stroke accelerates through it.
export function createWheelGesture() {
  let last = -Infinity;
  let x = 0;
  let y = 0;
  let fired = false;
  let firedAt = -Infinity;
  let previousMagnitude = 0;
  let tailMinimum = Infinity;
  let rising = 0;
  return (dx: number, dy: number, time: number): 1 | -1 | 0 => {
    const magnitude = Math.abs(dx);
    const reversed = fired && dx * x < 0 && magnitude >= 2;
    if (fired && time - firedAt > 80) {
      tailMinimum = Math.min(tailMinimum, magnitude);
      rising = magnitude > previousMagnitude ? rising + 1 : 0;
    }
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
