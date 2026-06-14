import hljs from "highlight.js/lib/core";
import javascript from "highlight.js/lib/languages/javascript";
import typescript from "highlight.js/lib/languages/typescript";
import rust from "highlight.js/lib/languages/rust";
import python from "highlight.js/lib/languages/python";
import go from "highlight.js/lib/languages/go";
import xml from "highlight.js/lib/languages/xml";
import css from "highlight.js/lib/languages/css";
import type { Highlighter } from "shiki";

hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("python", python);
hljs.registerLanguage("go", go);
hljs.registerLanguage("xml", xml);
hljs.registerLanguage("css", css);

const CANDIDATE_LANGS = [
  "typescript",
  "javascript",
  "rust",
  "python",
  "go",
  "xml",
  "css",
];

export function detectLanguage(text: string): string | null {
  if (text.trim().length < 10) return null;
  try {
    const result = hljs.highlightAuto(text, CANDIDATE_LANGS);
    if (result.language && result.relevance > 5) {
      return result.language;
    }
  } catch (_err) {}
  return null;
}

let highlighterPromise: Promise<Highlighter> | null = null;

function getHighlighter(): Promise<Highlighter> {
  if (!highlighterPromise) {
    highlighterPromise = import("shiki").then(({ createHighlighter }) =>
      createHighlighter({
        themes: ["github-dark"],
        langs: [
          "javascript",
          "typescript",
          "rust",
          "python",
          "go",
          "html",
          "css",
        ],
      }),
    );
  }
  return highlighterPromise;
}

export async function highlightCode(
  code: string,
  lang: string,
): Promise<string> {
  const hl = await getHighlighter();
  return hl.codeToHtml(code, { lang, theme: "github-dark" });
}
