import {
  Decoration,
  DecorationSet,
  EditorView,
  ViewPlugin,
  ViewUpdate,
  WidgetType,
} from '@codemirror/view';
import { RangeSetBuilder, StateField } from '@codemirror/state';
import { autocompletion, CompletionContext, CompletionResult } from '@codemirror/autocomplete';
import { syntaxTree } from '@codemirror/language';
import { invoke } from '@tauri-apps/api/core';

// Regex for matching [[wiki-links]]
const WIKI_LINK_REGEX = /\[\[([^\]|]+)(?:\|([^\]]+))?\]\]/g;

// Декоратор для подсветки wiki-links
class WikiLinkWidget extends WidgetType {
  constructor(readonly target: string, readonly alias?: string) {
    super();
  }

  eq(other: WikiLinkWidget) {
    return other.target === this.target && other.alias === this.alias;
  }

  toDOM() {
    const span = document.createElement('span');
    span.className = 'cm-wiki-link';
    span.textContent = this.alias || this.target;
    span.dataset.target = this.target;
    return span;
  }

  ignoreEvent() {
    return false;
  }
}

// Декорация для текста ссылки
const linkDecoration = Decoration.mark({ class: 'cm-wiki-link-text' });

function decorateWikiLinks(view: EditorView) {
  const builder = new RangeSetBuilder<Decoration>();
  const text = view.state.doc.toString();
  const cursor = view.state.selection.main.head;

  // Find all wiki-links
  let match;
  WIKI_LINK_REGEX.lastIndex = 0;

  while ((match = WIKI_LINK_REGEX.exec(text)) !== null) {
    const start = match.index;
    const end = start + match[0].length;
    const target = match[1];
    const alias = match[2];

    // НЕ декорировать если курсор внутри этой ссылки
    if (cursor >= start && cursor <= end) {
      continue;
    }

    // ПОЛНОСТЬЮ СКРЫТЬ открывающие скобки [[
    builder.add(start, start + 2, Decoration.replace({}));

    // Декорировать текст ссылки (target или alias)
    if (alias) {
      // Если есть alias: [[target|alias]] → показать только alias
      const pipePos = start + 2 + target.length;

      // Скрыть target
      builder.add(start + 2, pipePos, Decoration.replace({}));

      // Скрыть pipe |
      builder.add(pipePos, pipePos + 1, Decoration.replace({}));

      // Стилизовать alias
      builder.add(pipePos + 1, end - 2, linkDecoration);
    } else {
      // Если нет alias: [[target]] → показать target
      builder.add(start + 2, end - 2, linkDecoration);
    }

    // ПОЛНОСТЬЮ СКРЫТЬ закрывающие скобки ]]
    builder.add(end - 2, end, Decoration.replace({}));
  }

  return builder.finish();
}

// ViewPlugin для декорации
const wikiLinkDecorations = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = decorateWikiLinks(view);
    }

    update(update: ViewUpdate) {
      if (update.docChanged || update.viewportChanged || update.selectionSet) {
        this.decorations = decorateWikiLinks(update.view);
      }
    }
  },
  {
    decorations: (v) => v.decorations,
  }
);

// Ctrl+Click навигация
function handleClick(event: MouseEvent, view: EditorView): boolean {
  if (!event.ctrlKey && !event.metaKey) {
    return false;
  }

  const target = event.target as HTMLElement;

  // Check if clicked on wiki-link
  if (target.classList.contains('cm-wiki-link-text')) {
    event.preventDefault();

    // Find the wiki-link at this position
    const pos = view.posAtDOM(target);
    const text = view.state.doc.toString();

    // Find the surrounding [[...]]
    let start = pos;
    while (start > 0 && text.substring(start - 2, start) !== '[[') {
      start--;
    }

    let end = pos;
    while (end < text.length && text.substring(end, end + 2) !== ']]') {
      end++;
    }

    if (start > 0 && end < text.length) {
      const linkText = text.substring(start, end + 2);
      const match = linkText.match(/\[\[([^\]|]+)(?:\|([^\]]+))?\]\]/);

      if (match) {
        const linkTarget = match[1];
        navigateToWikiLink(linkTarget);
        return true;
      }
    }
  }

  return false;
}

// Navigate to wiki-link target
async function navigateToWikiLink(target: string) {
  try {
    const resolved = await invoke<string | null>('resolve_wiki_link', { target });
    if (resolved) {
      // Import notesStore dynamically to avoid circular dependency
      const { notesStore } = await import('../stores/notes.svelte');
      notesStore.openFile(resolved);
    } else {
      console.warn('Wiki link target not found:', target);
    }
  } catch (err) {
    console.error('Failed to resolve wiki link:', err);
  }
}

// Click event extension
const wikiLinkClickHandler = EditorView.domEventHandlers({
  mousedown: (event, view) => {
    return handleClick(event, view);
  },
});

// Autocomplete for wiki-links
async function wikiLinkCompletions(context: CompletionContext): Promise<CompletionResult | null> {
  const word = context.matchBefore(/\[\[([^\]]*)/);

  if (!word || (word.from === word.to && !context.explicit)) {
    return null;
  }

  // Get all notes from backend
  try {
    const { notesStore } = await import('../stores/notes.svelte');
    const notes = await invoke<any[]>('note_list', {});

    // Flatten notes to get all file names
    const flattenFiles = (nodes: any[]): string[] => {
      const files: string[] = [];
      for (const node of nodes) {
        if (!node.is_dir && node.name.endsWith('.md')) {
          const nameWithoutExt = node.name.replace(/\.md$/, '');
          files.push(nameWithoutExt);
        }
        if (node.children) {
          files.push(...flattenFiles(node.children));
        }
      }
      return files;
    };

    const fileNames = flattenFiles(notes);
    const query = word.text.substring(2); // Remove [[

    const options = fileNames
      .filter((name) => name.toLowerCase().includes(query.toLowerCase()))
      .map((name) => ({
        label: name,
        apply: (view: EditorView, completion: any, from: number, to: number) => {
          // Replace [[partial with [[name]]
          const insertText = `[[${name}]]`;
          view.dispatch({
            changes: { from: word.from, to: word.to, insert: insertText },
          });
        },
      }));

    return {
      from: word.from + 2, // After [[
      options,
      filter: false, // We already filtered
    };
  } catch (err) {
    console.error('Failed to load completions:', err);
    return null;
  }
}

// Export combined extension
export function wikiLinksExtension() {
  return [
    wikiLinkDecorations,
    wikiLinkClickHandler,
    autocompletion({
      override: [wikiLinkCompletions],
    }),
  ];
}
