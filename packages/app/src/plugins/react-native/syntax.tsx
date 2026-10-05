import { isLanguageSupported, type HighlightToken } from "@getpaseo/highlight";
import { Text } from "react-native";
import { syntaxTokenStyleFor } from "@/styles/syntax-token-styles";
import { extensionFromPath, tokenizeToLines } from "@/utils/highlight-cache";

export function tokenizeCode(code: string, filePath: string): HighlightToken[][] | null {
  const ext = extensionFromPath(filePath);
  if (!ext || !isLanguageSupported(`x.${ext}`)) return null;
  return tokenizeToLines(code, ext);
}

// Colors come from Unistyles, so a theme switch recolors tokens without re-tokenizing.
export function SyntaxToken({ token }: { token: { text: string; style: string | null } }) {
  return <Text style={syntaxTokenStyleFor(token.style)}>{token.text}</Text>;
}
