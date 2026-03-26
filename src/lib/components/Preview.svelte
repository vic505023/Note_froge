<script lang="ts">
  import { renderMarkdown } from '../utils/markdown';
  import { notesStore } from '../stores/notes.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { confirm } from '@tauri-apps/plugin-dialog';

  let { content = '' } = $props<{ content: string }>();
  let previewElement: HTMLDivElement;

  // Рендер markdown в HTML
  let html = $derived(renderMarkdown(content));

  // Обработка кликов на ссылки
  function handleClick(event: MouseEvent) {
    const target = event.target as HTMLElement;

    // Task list checkboxes
    if (target.tagName === 'INPUT' && target.getAttribute('type') === 'checkbox') {
      event.preventDefault();
      handleCheckboxToggle(target as HTMLInputElement);
      return;
    }

    // Wiki-links
    if (target.classList.contains('wiki-link')) {
      event.preventDefault();
      const linkTarget = target.getAttribute('data-target');
      if (linkTarget) {
        resolveAndOpenWikiLink(linkTarget);
      }
      return;
    }

    // Internal links (.md files)
    if (target.classList.contains('internal-link')) {
      event.preventDefault();
      const linkTarget = target.getAttribute('data-target');
      if (linkTarget) {
        openInternalLink(linkTarget);
      }
      return;
    }

    // External links - открываем в браузере
    if (target.classList.contains('external-link')) {
      event.preventDefault();
      const href = target.getAttribute('href');
      if (href) {
        invoke('open_external_url', { url: href }).catch(err => {
          console.error('Failed to open URL:', err);
        });
      }
      return;
    }
  }

  async function handleCheckboxToggle(checkbox: HTMLInputElement) {
    const taskText = checkbox.getAttribute('data-task-text');
    if (!taskText || !notesStore.currentFile) return;

    try {
      // Читаем текущий контент файла
      const currentContent = notesStore.currentContent;
      const lines = currentContent.split('\n');

      // Ищем строку с этой задачей
      const pattern = checkbox.checked
        ? `- [ ] ${taskText}`
        : `- [x] ${taskText}`;
      const replacement = checkbox.checked
        ? `- [x] ${taskText}`
        : `- [ ] ${taskText}`;

      let found = false;
      const newLines = lines.map(line => {
        if (!found && line.trim() === pattern.trim()) {
          found = true;
          return line.replace(pattern, replacement);
        }
        return line;
      });

      if (found) {
        const newContent = newLines.join('\n');
        await invoke('note_write', {
          path: notesStore.currentFile,
          content: newContent
        });

        // Обновляем store
        notesStore.updateContent(newContent);
      }
    } catch (err) {
      console.error('Failed to toggle checkbox:', err);
      // Возвращаем состояние чекбокса
      checkbox.checked = !checkbox.checked;
    }
  }

  async function openInternalLink(path: string) {
    try {
      // Проверяем существование файла
      const exists = await invoke<boolean>('note_exists', { path });
      if (exists) {
        notesStore.openFile(path);
      } else {
        // Заметка не найдена - предложить создать
        const shouldCreate = await confirm(
          `Note "${path}" not found. Create it?`,
          { title: 'Create Note', kind: 'info' }
        );
        if (shouldCreate) {
          await notesStore.createFile(path);
        }
      }
    } catch (err) {
      console.error('Failed to open internal link:', err);
    }
  }

  async function resolveAndOpenWikiLink(target: string) {
    try {
      const resolved = await invoke<string | null>('resolve_wiki_link', { target });
      if (resolved) {
        notesStore.openFile(resolved);
      } else {
        // Заметка не найдена - предложить создать
        const shouldCreate = await confirm(
          `Note "${target}" not found. Create it?`,
          { title: 'Create Note', kind: 'info' }
        );
        if (shouldCreate) {
          const path = target.endsWith('.md') ? target : `${target}.md`;
          await notesStore.createFile(path);
        }
      }
    } catch (err) {
      console.error('Failed to resolve wiki link:', err);
    }
  }
</script>

<div
  class="preview"
  bind:this={previewElement}
  onclick={handleClick}
  role="article"
>
  {@html html}
</div>

<style>
  .preview {
    width: 100%;
    max-width: 800px;
    margin: 0 auto;
    padding: 0;
    background: transparent;
    color: var(--text-primary);
    font-family: 'Inter', system-ui, -apple-system, sans-serif;
    font-size: 15px;
    line-height: 1.7;
  }

  /* Параграфы */
  :global(.preview p) {
    margin: 0 0 0.8em 0;
  }

  /* Заголовки */
  :global(.preview h1) {
    color: var(--text-primary);
    font-size: 1.8em;
    font-weight: 700;
    line-height: 1.25;
    letter-spacing: -0.02em;
    margin: 1.2em 0 0.6em 0;
  }

  :global(.preview h1:first-child) {
    margin-top: 0;
  }

  :global(.preview h2) {
    color: var(--text-primary);
    font-size: 1.5em;
    font-weight: 600;
    line-height: 1.35;
    letter-spacing: -0.01em;
    margin: 1em 0 0.5em 0;
  }

  :global(.preview h3) {
    color: var(--text-primary);
    font-size: 1.25em;
    font-weight: 600;
    line-height: 1.4;
    margin: 0.8em 0 0.4em 0;
  }

  :global(.preview h4) {
    color: var(--text-primary);
    font-size: 1.1em;
    font-weight: 600;
    margin: 0.6em 0 0.3em 0;
  }

  :global(.preview h5),
  :global(.preview h6) {
    color: var(--text-primary);
    font-weight: 600;
    margin: 0.5em 0 0.2em 0;
  }

  /* Ссылки */
  :global(.preview a.wiki-link) {
    color: var(--link-wiki);
    text-decoration: underline;
    text-decoration-style: dashed;
    text-decoration-thickness: 1px;
    text-underline-offset: 2px;
    cursor: pointer;
    transition: color var(--transition-fast);
  }

  :global(.preview a.wiki-link:hover) {
    color: var(--accent-hover);
  }

  :global(.preview a.wiki-link.not-found) {
    color: var(--error);
  }

  :global(.preview a.internal-link) {
    color: var(--link-url);
    text-decoration: underline;
    cursor: pointer;
    transition: color var(--transition-fast);
  }

  :global(.preview a.internal-link:hover) {
    color: var(--accent-hover);
  }

  :global(.preview a.external-link) {
    color: var(--link-url);
    text-decoration: underline;
    cursor: pointer;
  }

  :global(.preview a.external-link::after) {
    content: ' ↗';
    font-size: 0.8em;
    opacity: 0.6;
  }

  :global(.preview a.external-link:hover) {
    color: var(--accent-hover);
  }

  /* Inline код */
  :global(.preview code:not(.hljs code)) {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 2px 6px;
    color: var(--accent-hover);
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.9em;
  }

  /* Блоки кода */
  :global(.preview pre.hljs) {
    position: relative;
    background: var(--bg-secondary);
    border-radius: 6px;
    padding: 16px;
    overflow-x: auto;
    margin: 1em 0;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.9em;
    line-height: 1.5;
  }

  /* Язык программирования в углу */
  :global(.preview pre.hljs[data-lang]::after) {
    content: attr(data-lang);
    position: absolute;
    top: 8px;
    right: 12px;
    background: rgba(0, 0, 0, 0.3);
    color: var(--text-secondary);
    padding: 2px 8px;
    border-radius: 3px;
    font-size: 0.75em;
    font-family: 'JetBrains Mono', monospace;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  :global(.preview pre.hljs code) {
    background: transparent;
    border: none;
    padding: 0;
    color: var(--text-primary);
  }

  /* Списки */
  :global(.preview ul),
  :global(.preview ol) {
    margin: 0.8em 0;
    padding-left: 2em;
  }

  :global(.preview li) {
    margin: 0.3em 0;
  }

  :global(.preview li.task-list-item) {
    list-style: none;
    margin-left: -1.5em;
  }

  :global(.preview li.task-list-item input[type="checkbox"]) {
    width: 18px;
    height: 18px;
    min-width: 18px;
    max-width: 18px;
    margin: 0 0.5em 0 0;
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

  :global(.preview li.task-list-item input[type="checkbox"]:hover) {
    border-color: var(--accent);
  }

  :global(.preview li.task-list-item input[type="checkbox"]:checked) {
    background: var(--accent);
    border-color: var(--accent);
  }

  :global(.preview li.task-list-item input[type="checkbox"]:checked::after) {
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

  /* Цитаты */
  :global(.preview blockquote) {
    border-left: 3px solid var(--accent);
    padding-left: 16px;
    margin: 1em 0;
    font-style: italic;
    color: var(--text-secondary);
  }

  /* Таблицы */
  :global(.preview table) {
    border-collapse: collapse;
    width: 100%;
    margin: 1em 0;
    border: 1px solid var(--border);
  }

  :global(.preview th) {
    background: var(--bg-elevated);
    font-weight: 600;
    text-align: left;
    padding: 8px 12px;
    border: 1px solid var(--border);
  }

  :global(.preview td) {
    padding: 8px 12px;
    border: 1px solid var(--border);
  }

  :global(.preview tr:nth-child(even)) {
    background: rgba(255, 255, 255, 0.02);
  }

  /* Горизонтальная линия */
  :global(.preview hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 2em 0;
  }

  /* Жирный и курсив */
  :global(.preview strong) {
    font-weight: 600;
    color: var(--text-primary);
  }

  :global(.preview em) {
    font-style: italic;
  }

  /* Изображения */
  :global(.preview img) {
    max-width: 100%;
    height: auto;
    border-radius: 6px;
    margin: 1em 0;
  }

</style>
