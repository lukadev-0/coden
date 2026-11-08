import { EditorView } from "@codemirror/view";
import { type Extension } from "@codemirror/state";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";

export const theme = EditorView.theme(
  {
    "&": {
      color: "var(--color-on-base-dim)",
      backgroundColor: "var(--color-surface)",
    },
    "&.cm-focused": {
      outline: "none",
    },
    ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--color-accent)" },
    ".cm-panels": { backgroundColor: "var(--color-container)" },
    ".cm-searchMatch": {
      backgroundColor: "var(--color-accent-container)",
      color: "var(--color-on-accent-container)",
    },
    ".cm-searchMatch.cm-searchMatch-selected": {
      backgroundColor: "var(--color-accent)",
      color: "var(--color-on-accent)",
    },
    ".cm-activeLine": { backgroundColor: "var(--color-editor-active-line)" },
    ".cm-selectionMatch": { backgroundColor: "var(--color-accent-container)" },
    ".cm-gutters": {
      backgroundColor: "var(--color-surface)",
      color: "var(--color-on-surface)",
    },
    ".cm-activeLineGutter": {
      backgroundColor: "var(--color-accent-container)",
    },
  },
  { dark: true },
);

export const highlightStyle = HighlightStyle.define([
  { tag: t.keyword, color: "var(--color-code-keyword)" },
  {
    tag: [t.name, t.deleted, t.character, t.propertyName, t.macroName],
    color: "var(--color-code-name)",
  },
  { tag: [t.color, t.constant(t.name), t.standard(t.name)], color: "var(--color-code-builtin)" },
  {
    tag: [t.number],
    color: "var(--color-code-number)",
  },
  { tag: [t.meta, t.comment], color: "var(--color-code-comment)" },
  { tag: [t.atom, t.bool, t.special(t.variableName)], color: "var(--color-code-bool)" },
  { tag: [t.processingInstruction, t.string], color: "var(--color-code-string)" },
]);

export const editorTheme: Extension = [theme, syntaxHighlighting(highlightStyle)];
