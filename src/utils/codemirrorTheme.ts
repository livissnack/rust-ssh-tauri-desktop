import { EditorView } from '@codemirror/view';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags as t } from '@lezer/highlight';

export const appCodemirrorTheme = EditorView.theme(
  {
    '&': {
      height: '100%',
      fontSize: '13px',
    },
    '.cm-scroller': {
      overflow: 'auto',
      fontFamily:
        'ui-monospace, SFMono-Regular, "Cascadia Code", Menlo, Monaco, Consolas, monospace',
      lineHeight: '1.6',
    },
    '.cm-content': {
      caretColor: 'var(--accent)',
      padding: '10px 0',
    },
    '.cm-cursor, .cm-dropCursor': {
      borderLeftColor: 'var(--accent)',
      borderLeftWidth: '2px',
    },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection': {
      backgroundColor: 'var(--accent-20, rgba(127, 127, 127, 0.22)) !important',
    },
    '.cm-activeLine': {
      backgroundColor: 'var(--accent-05, rgba(127, 127, 127, 0.06))',
    },
    '.cm-gutters': {
      backgroundColor: 'var(--bg-secondary)',
      color: 'var(--text-dim)',
      border: 'none',
      borderRight: '1px solid var(--border)',
      paddingRight: '4px',
    },
    '.cm-activeLineGutter': {
      backgroundColor: 'var(--accent-08, rgba(127, 127, 127, 0.08))',
      color: 'var(--text-main)',
    },
    '.cm-foldPlaceholder': {
      backgroundColor: 'transparent',
      border: 'none',
      color: 'var(--text-dim)',
    },
    '.cm-matchingBracket, .cm-nonmatchingBracket': {
      backgroundColor: 'var(--accent-10, rgba(127, 127, 127, 0.12))',
      outline: '1px solid var(--accent-30, rgba(127, 127, 127, 0.3))',
    },
    '.cm-panels': {
      backgroundColor: 'var(--bg-secondary)',
      color: 'var(--text-main)',
    },
    '.cm-panels-bottom': {
      borderTop: '1px solid var(--border)',
    },
    '.cm-panel.cm-search': {
      padding: '10px 12px 10px',
      backgroundColor: 'var(--bg-secondary)',
    },
    '.cm-search': {
      display: 'flex',
      flexWrap: 'wrap',
      alignItems: 'center',
      gap: '6px 8px',
      paddingRight: '32px',
      position: 'relative',
    },
    '.cm-search .cm-textfield': {
      height: '28px',
      minWidth: '160px',
      padding: '0 10px',
      borderRadius: '6px',
      border: '1px solid var(--border)',
      backgroundColor: 'var(--bg-input, var(--bg-card))',
      color: 'var(--text-main)',
      fontSize: '12px',
      fontFamily: 'inherit',
      outline: 'none',
    },
    '.cm-search .cm-textfield:focus': {
      borderColor: 'var(--accent)',
      boxShadow: '0 0 0 2px var(--accent-15, rgba(127, 127, 127, 0.15))',
    },
    '.cm-search .cm-button': {
      height: '28px',
      padding: '0 10px',
      borderRadius: '6px',
      border: '1px solid var(--border)',
      backgroundColor: 'var(--bg-card)',
      color: 'var(--text-main)',
      fontSize: '11px',
      fontWeight: '600',
      cursor: 'pointer',
      fontFamily: 'inherit',
    },
    '.cm-search .cm-button:hover': {
      backgroundColor: 'var(--accent-08, var(--bg-secondary))',
      borderColor: 'var(--accent-30, var(--border))',
      color: 'var(--text-main)',
    },
    '.cm-search label': {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '4px',
      margin: '0',
      fontSize: '11px',
      fontWeight: '500',
      color: 'var(--text-dim)',
      whiteSpace: 'nowrap',
      cursor: 'pointer',
    },
    '.cm-search input[type=checkbox]': {
      width: '14px',
      height: '14px',
      margin: '0',
      accentColor: 'var(--accent)',
      cursor: 'pointer',
    },
    '.cm-search button[name=close]': {
      position: 'absolute',
      top: '0',
      right: '0',
      width: '26px',
      height: '26px',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      padding: '0',
      margin: '0',
      borderRadius: '6px',
      border: '1px solid transparent',
      backgroundColor: 'transparent',
      color: 'var(--text-dim)',
      fontSize: '16px',
      lineHeight: '1',
      cursor: 'pointer',
    },
    '.cm-search button[name=close]:hover': {
      backgroundColor: 'var(--bg-card)',
      borderColor: 'var(--border)',
      color: 'var(--text-main)',
    },
    '.cm-searchMatch': {
      backgroundColor: 'var(--accent-20, rgba(127, 127, 127, 0.2))',
      borderRadius: '2px',
    },
    '.cm-searchMatch.cm-searchMatch-selected': {
      backgroundColor: 'var(--accent-30, rgba(127, 127, 127, 0.3))',
      outline: '1px solid var(--accent)',
    },
  },
  { dark: true },
);

export const appCodemirrorHighlight = syntaxHighlighting(
  HighlightStyle.define([
    { tag: t.keyword, color: 'var(--accent-alt)' },
    { tag: [t.string, t.special(t.string)], color: 'var(--success)' },
    { tag: t.comment, color: 'var(--text-dim)', fontStyle: 'italic' },
    { tag: [t.number, t.bool, t.atom], color: 'var(--accent-orange)' },
    { tag: t.function(t.variableName), color: 'var(--accent)' },
    { tag: [t.typeName, t.className], color: 'var(--accent-alt)' },
    { tag: t.operator, color: 'var(--text-main)' },
    { tag: t.punctuation, color: 'var(--text-dim)' },
    { tag: t.propertyName, color: 'var(--text-main)' },
    { tag: t.tagName, color: 'var(--error)' },
    { tag: t.attributeName, color: 'var(--accent-orange)' },
    { tag: t.meta, color: 'var(--text-dim)' },
    { tag: t.invalid, color: 'var(--error)', textDecoration: 'underline wavy' },
  ]),
);
