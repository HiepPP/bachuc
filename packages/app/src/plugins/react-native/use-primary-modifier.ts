import { useEffect, useState } from "react";
import { isWeb } from "@/constants/platform";

/** Resets on blur so switching apps while holding a modifier cannot leave an action swapped. */
export function usePrimaryModifier(): boolean {
  const [pressed, setPressed] = useState(false);
  useEffect(() => {
    if (!isWeb) return;
    const mac = /Mac|iPhone|iPad/.test(navigator.platform);
    const update = (event: KeyboardEvent | MouseEvent) =>
      setPressed(mac ? event.metaKey : event.ctrlKey);
    const clear = () => setPressed(false);
    window.addEventListener("keydown", update);
    window.addEventListener("keyup", update);
    window.addEventListener("mousemove", update);
    window.addEventListener("blur", clear);
    return () => {
      window.removeEventListener("keydown", update);
      window.removeEventListener("keyup", update);
      window.removeEventListener("mousemove", update);
      window.removeEventListener("blur", clear);
    };
  }, []);
  return pressed;
}
