<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Transaction } from '@codemirror/state';
  import type { EditorView } from '@codemirror/view';
  import { createEditorState, createEditorView } from '../utils/codemirror';
  import { notesStore } from '../stores/notes.svelte';
  import { settingsStore } from '../stores/settings.svelte';

  let editorElement: HTMLDivElement;
  let editorView: EditorView | null = null;
  let saveTimeout: ReturnType<typeof setTimeout> | null = null;
  let currentContent = $state(notesStore.currentContent);

  function handleChange(value: string) {
    currentContent = value;
    notesStore.updateContent(value);

    if (saveTimeout !== null) {
      clearTimeout(saveTimeout);
    }

    saveTimeout = setTimeout(() => {
      notesStore.saveFile();
    }, 1000);
  }

  onMount(() => {
    const state = createEditorState(currentContent, handleChange);
    editorView = createEditorView(editorElement, state);
    applyEditorSettings();
  });

  function applyEditorSettings() {
    if (!editorView || !settingsStore.config) return;

    const root = document.documentElement;
    root.style.setProperty('--editor-font-size', `${settingsStore.config.editor.font_size}px`);
    root.style.setProperty('--editor-font-family', settingsStore.config.editor.font_family);
  }

  // Watch for settings changes
  $effect(() => {
    if (settingsStore.config) {
      applyEditorSettings();
    }
  });

  $effect(() => {
    const newContent = notesStore.currentContent;
    const isAIEdit = notesStore.aiEditPending;

    currentContent = newContent;

    if (!editorView) return;

    const currentDoc = editorView.state.doc.toString();
    if (currentDoc === newContent) return;

    editorView.dispatch({
      changes: {
        from: 0,
        to: currentDoc.length,
        insert: newContent
      },
      annotations: [
        isAIEdit
          ? Transaction.addToHistory.of(true)
          : Transaction.addToHistory.of(false)
      ]
    });

    if (isAIEdit) {
      notesStore.clearAIEditFlag();
    }
  });

  onDestroy(() => {
    if (saveTimeout !== null) {
      clearTimeout(saveTimeout);
      saveTimeout = null;
    }

    if (editorView) {
      editorView.destroy();
      editorView = null;
    }
  });
</script>

<div class="editor" bind:this={editorElement}></div>

<style>
  .editor {
    width: 100%;
    max-width: 800px;
    margin: 0 auto;
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
  }

  /* CodeMirror styling */
  :global(.cm-editor) {
    background: transparent !important;
    color: var(--text-primary) !important;
    font-family: var(--editor-font-family);
    font-size: var(--editor-font-size);
    line-height: 1.75;
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
  }

  :global(.cm-scroller) {
    overflow: auto;
    font-family: inherit;
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
  }

  /* Hide scrollbars in editor */
  :global(.cm-scroller::-webkit-scrollbar) {
    display: none;
  }

  :global(.cm-scroller) {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }

  :global(.cm-content) {
    padding: 0;
    caret-color: var(--accent) !important;
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
  }

  :global(.cm-line) {
    padding: 3px 0;
  }

  /* Выделение текста */
  :global(.cm-selectionBackground) {
    background: rgba(76, 139, 107, 0.25) !important;
  }

  :global(.cm-focused .cm-selectionBackground) {
    background: rgba(76, 139, 107, 0.30) !important;
  }

  /* Цвет текста внутри выделения */
  :global(.cm-content ::selection) {
    background: rgba(76, 139, 107, 0.30) !important;
    color: var(--text-primary) !important;
  }

  :global(.cm-line ::selection) {
    background: rgba(76, 139, 107, 0.30) !important;
    color: var(--text-primary) !important;
  }

  /* Убрать outline при фокусе */
  :global(.cm-editor.cm-focused) {
    outline: none !important;
    border: none !important;
  }

  /* Курсор */
  :global(.cm-cursor, .cm-cursor-primary) {
    border-left-color: var(--accent) !important;
    border-left-width: 2px !important;
  }

  /* Активная строка */
  :global(.cm-activeLine) {
    background: rgba(255, 255, 255, 0.02) !important;
  }

  /* Markdown заголовки */
  :global(.cm-line .ͼ1) {
    color: var(--text-primary);
    font-size: 2rem;
    font-weight: 700;
    line-height: 1.25;
    letter-spacing: -0.02em;
  }

  :global(.cm-line .ͼ2) {
    color: var(--text-primary);
    font-size: 1.5rem;
    font-weight: 600;
    line-height: 1.35;
    letter-spacing: -0.01em;
  }

  :global(.cm-line .ͼ3) {
    color: var(--text-primary);
    font-size: 1.25rem;
    font-weight: 600;
    line-height: 1.4;
  }

  /* Markdown inline элементы */
  :global(.cm-strong) {
    color: var(--text-primary);
    font-weight: 600;
  }

  :global(.cm-em) {
    color: var(--text-primary);
    font-style: italic;
  }

  :global(.cm-link) {
    color: var(--accent);
    text-decoration: underline;
    text-decoration-color: var(--accent-soft);
  }

  :global(.cm-url) {
    color: var(--accent-hover);
    opacity: 0.7;
  }

  /* Код */
  :global(.cm-monospace) {
    background: var(--code-bg);
    border: 1px solid var(--code-border);
    border-radius: 3px;
    padding: 2px 4px;
    color: var(--accent-hover);
    font-size: 0.9em;
  }

  /* Списки */
  :global(.cm-list) {
    color: var(--accent);
  }

  /* Gutters */
  :global(.cm-gutters) {
    background: transparent !important;
    border-right: none;
    color: var(--text-muted);
  }

  :global(.cm-lineNumbers .cm-gutterElement) {
    padding: 0 12px 0 8px;
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  /* Wiki-links */
  :global(.cm-wiki-link-text) {
    color: var(--link-wiki);
    cursor: pointer;
    transition: color var(--transition-fast);
  }

  :global(.cm-wiki-link-text:hover) {
    color: var(--accent-hover);
    text-decoration: underline;
    text-decoration-style: dashed;
  }

  :global(.cm-wiki-bracket) {
    color: var(--text-muted);
    opacity: 0.6;
  }

  /* List bullets */
  :global(.cm-list-bullet) {
    color: var(--accent);
    margin-right: 0.5em;
    font-weight: bold;
    user-select: none;
  }

  /* Code blocks */
  :global(.cm-line.cm-code-block-line) {
    background: var(--bg-secondary);
    padding: 2px 8px;
  }

  /* Rounded corners for code blocks */
  :global(.cm-line.cm-code-block-line) {
    border-radius: 0;
  }

  /* First line */
  :global(.cm-line.cm-code-block-line:not(.cm-code-block-line ~ .cm-code-block-line)) {
    border-top-left-radius: 6px;
    border-top-right-radius: 6px;
  }

  /* Last line - fallback for single line */
  :global(.cm-line.cm-code-block-line:not(:has(~ .cm-code-block-line))) {
    border-bottom-left-radius: 6px;
    border-bottom-right-radius: 6px;
  }

  /* Single line block */
  :global(.cm-line.cm-code-block-line:only-child) {
    border-radius: 6px;
  }

  :global(.cm-code-block-text) {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.92em;
    color: var(--text-primary);
  }

  /* Hidden code block fences */
  :global(.cm-line.cm-code-fence-hidden) {
    height: 0 !important;
    overflow: hidden !important;
    line-height: 0 !important;
    padding: 0 !important;
    margin: 0 !important;
  }

  /* Task list checkboxes */
  :global(.cm-task-checkbox-wrapper) {
    display: inline-block;
    margin-right: 0.5em;
    vertical-align: middle;
  }

  :global(.cm-task-checkbox) {
    width: 18px;
    height: 18px;
    min-width: 18px;
    max-width: 18px;
    margin: 0;
    padding: 0;
    cursor: pointer;
    appearance: none;
    -webkit-appearance: none;
    border: 2px solid var(--border);
    border-radius: 4px;
    background: transparent;
    vertical-align: middle;
    position: relative;
    transition: all 0.2s ease;
    flex-shrink: 0;
  }

  :global(.cm-task-checkbox:hover) {
    border-color: var(--accent);
  }

  :global(.cm-task-checkbox:checked) {
    background: var(--accent);
    border-color: var(--accent);
  }

  :global(.cm-task-checkbox:checked::after) {
    content: '';
    position: absolute;
    left: 5px;
    top: 2px;
    width: 4px;
    height: 8px;
    border: solid var(--bg-primary);
    border-width: 0 2px 2px 0;
    transform: rotate(45deg);
  }
</style>