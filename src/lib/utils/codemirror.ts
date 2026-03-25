import { EditorState } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap, undo, redo } from '@codemirror/commands';
import { markdown } from '@codemirror/lang-markdown';
import { oneDark } from '@codemirror/theme-one-dark';
import { wikiLinksExtension } from './wikilinks-cm';

// Custom keymap that works with any keyboard layout
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

    // Undo: Ctrl+Z (works on any layout)
    if (event.ctrlKey && event.code === 'KeyZ' && !event.shiftKey) {
      event.preventDefault();
      undo(view);
      return true;
    }
    // Redo: Ctrl+Shift+Z or Ctrl+Y (works on any layout)
    if ((event.ctrlKey && event.code === 'KeyZ' && event.shiftKey) ||
        (event.ctrlKey && event.code === 'KeyY')) {
      event.preventDefault();
      redo(view);
      return true;
    }
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
      layoutIndependentKeymap,
      keymap.of([...historyKeymap, ...defaultKeymap]),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          onChange(update.state.doc.toString());
        }
      }),
      EditorView.lineWrapping,
      wikiLinksExtension(), // Wiki-links support
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
