import type { ReactNode } from "react";
import { StyleSheet, View, type LayoutChangeEvent } from "react-native";
import Animated, {
  Easing,
  makeMutable,
  useAnimatedStyle,
  withSequence,
  withTiming,
} from "react-native-reanimated";

// One sidebar list per window, so the slide state is module-wide. `offset` is a fraction of the
// list width: -1 is fully off to the left, 1 fully off to the right.
const offset = makeMutable(0);
const opacity = makeMutable(1);
const width = makeMutable(0);
const OUT = { duration: 65, easing: Easing.bezier(0.4, 0, 1, 1) };
const IN = { duration: 105, easing: Easing.bezier(0.16, 1, 0.3, 1) };
let pending: { timer: ReturnType<typeof setTimeout>; apply: () => void } | null = null;

/**
 * Slides the list out against `direction`, applies the change, then slides the new list in from
 * `direction`. A second slide during the first applies the first change at once.
 */
export function slidePluginSidebar(direction: 1 | -1, apply: () => void): void {
  finishPendingSlide();
  offset.value = withTiming(-direction, OUT);
  opacity.value = withTiming(0, OUT);
  const timer = setTimeout(() => {
    pending = null;
    apply();
    offset.value = withSequence(withTiming(direction, { duration: 0 }), withTiming(0, IN));
    opacity.value = withTiming(1, IN);
  }, OUT.duration);
  pending = { timer, apply };
}

function finishPendingSlide(): void {
  if (!pending) return;
  clearTimeout(pending.timer);
  const { apply } = pending;
  pending = null;
  apply();
}

function handleLayout(event: LayoutChangeEvent): void {
  width.value = event.nativeEvent.layout.width;
}

/** Wraps the sidebar list so plugin filter changes can slide it. */
export function PluginSidebarSlide({ children }: { children: ReactNode }) {
  // Offset with `left`, not a transform: a resting transform would make this view the containing
  // block for fixed-position descendants. Return the same keys every frame; web keeps a key the
  // style stops returning at its last value.
  const animatedStyle = useAnimatedStyle(() => ({
    opacity: opacity.value,
    left: offset.value * width.value,
  }));
  return (
    <View style={styles.clip} onLayout={handleLayout}>
      <Animated.View style={[styles.fill, animatedStyle]}>{children}</Animated.View>
    </View>
  );
}

// Static styles for Animated.Views; Unistyles dynamic styles do not mix with animated styles.
const styles = StyleSheet.create({
  clip: { flex: 1, overflow: "hidden" },
  fill: { flex: 1 },
});
