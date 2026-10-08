import { useMemo } from "react";
import { Image } from "react-native";

interface PaseoLogoProps {
  size?: number;
  // Kept for callers; the Bachuc logo is a full-color image, so it ignores the color.
  color?: string;
}

// The Ba Chuc tree on its green rounded square, the same image as the app icon.
const LOGO_SOURCE = require("../../../assets/images/icon.png");

export function PaseoLogo({ size = 64 }: PaseoLogoProps) {
  const style = useMemo(() => ({ width: size, height: size }), [size]);
  return <Image source={LOGO_SOURCE} style={style} accessibilityIgnoresInvertColors />;
}
