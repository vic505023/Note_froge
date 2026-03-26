import { EditorState } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import { defaultKeymap, history, undo, redo } from '@codemirror/commands';
import { markdown } from '@codemirror/lang-markdown';
import { oneDark } from '@codemirror/theme-one-dark';
import { wikiLinksExtension } from './wikilinks-cm';
import { liveMarkdown } from './live-markdown';
import { liveMarkdownSimple } from './live-markdown-simple';
import { markdownKeymap } from './markdown-keymap';

// Layout-independent keyboard handler using event.code instead of event.key
const layoutIndependentKeymap = EditorView.domEventHandlers({
  keydown(event, view) {
    // Pass through global shortcuts to window handlers
    // Simple Ctrl+Key combinations (no shift/alt)
    const simpleGlobalShortcuts = ['KeyP', 'KeyE', 'KeyN', 'KeyB', 'KeyS', 'Comma'];
    if (event.ctrlKey && !event.shiftKey && !event.altKey && simpleGlobalShortcuts.includes(event.code)) {
      // Let the global handler in Layout.svelte handle these
      return false;
    }

    // Ctrl+Shift combinations
    if (event.ctrlKey && event.shiftKey && !event.altKey) {
      if (event.code === 'KeyF' || event.code === 'KeyA') {
        // Let the global handler in Layout.svelte handle these
        return false;
      }
    }

    // Undo: Ctrl+Z (works on any keyboard layout)
    if (event.ctrlKey && event.code === 'KeyZ' && !event.shiftKey && !event.altKey) {
      event.preventDefault();
      undo(view);
      return true;
    }

    // Redo: Ctrl+Shift+Z or Ctrl+Y (works on any keyboard layout)
    if (event.ctrlKey && event.shiftKey && event.code === 'KeyZ' && !event.altKey) {
      event.preventDefault();
      redo(view);
      return true;
    }

    if (event.ctrlKey && event.code === 'KeyY' && !event.shiftKey && !event.altKey) {
      event.preventDefault();
      redo(view);
      return true;
    }

    // Don't interfere with any other keys - let CodeMirror handle them
    return false;
  }
});

export function createEditorState(
  content: string,
  onChange: (value: string) => void
): EditorState {
  return EditorState.create({
    doc: content,
    extensions: [
      markdown(),
      oneDark,
      history(),
      // Layout-independent keyboard handler (handles Undo/Redo and global shortcuts)
      layoutIndependentKeymap,
      // Markdown keymap (списки, чекбоксы, Tab/Shift-Tab)
      markdownKeymap,
      // Default keymap for all other operations (Ctrl+A, Backspace, etc.)
      keymap.of(defaultKeymap),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          onChange(update.state.doc.toString());
        }
      }),
      EditorView.lineWrapping,
      wikiLinksExtension(), // Wiki-links support
      liveMarkdownSimple, // ТЕСТ: упрощенная версия
      // liveMarkdown, // Live markdown rendering (ПОСЛЕ wiki-links чтобы не конфликтовать)
    ],
  });
}

export function createEditorView(
  parent: HTMLElement,
  state: EditorState
): EditorView {
  return new EditorView({
    state,
    parent,
  });
}
