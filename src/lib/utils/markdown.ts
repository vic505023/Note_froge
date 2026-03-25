import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js';
import type StateInline from 'markdown-it/lib/rules_inline/state_inline';

// Настройка highlight.js с популярными языками
hljs.configure({
  languages: ['javascript', 'typescript', 'python', 'rust', 'bash', 'json', 'html', 'css', 'sql', 'yaml', 'toml', 'go', 'c', 'cpp', 'java']
});

// Кастомный плагин для wiki-links
function wikiLinksPlugin(md: MarkdownIt) {
  // Inline rule для парсинга [[target]] и [[target|alias]]
  function wikiLinkRule(state: StateInline, silent: boolean): boolean {
    const start = state.pos;
    const max = state.posMax;

    // Проверка на [[
    if (state.src.charCodeAt(start) !== 0x5B /* [ */ ||
        state.src.charCodeAt(start + 1) !== 0x5B /* [ */) {
      return false;
    }

    // Поиск закрывающих ]]
    let pos = start + 2;
    let found = false;
    while (pos < max - 1) {
      if (state.src.charCodeAt(pos) === 0x5D /* ] */ &&
          state.src.charCodeAt(pos + 1) === 0x5D /* ] */) {
        found = true;
        break;
      }
      pos++;
    }

    if (!found) return false;

    const content = state.src.slice(start + 2, pos);
    if (content.trim().length === 0) return false;

    if (!silent) {
      // Парсинг target и alias
      const parts = content.split('|');
      const target = parts[0].trim();
      const alias = parts.length > 1 ? parts[1].trim() : target;

      const token = state.push('wiki_link', '', 0);
      token.content = alias;
      token.meta = { target };
    }

    state.pos = pos + 2;
    return true;
  }

  md.inline.ruler.before('link', 'wiki_link', wikiLinkRule);

  // Renderer для wiki-links
  md.renderer.rules.wiki_link = (tokens, idx) => {
    const token = tokens[idx];
    const target = token.meta.target;
    const alias = token.content;
    return `<a class="wiki-link" data-target="${md.utils.escapeHtml(target)}">${md.utils.escapeHtml(alias)}</a>`;
  };
}

// Кастомный renderer для внутренних и внешних ссылок
function customLinkRenderer(md: MarkdownIt) {
  const defaultRender = md.renderer.rules.link_open || function(tokens, idx, options, env, self) {
    return self.renderToken(tokens, idx, options);
  };

  md.renderer.rules.link_open = (tokens, idx, options, env, self) => {
    const token = tokens[idx];
    const hrefIndex = token.attrIndex('href');

    if (hrefIndex >= 0) {
      const href = token.attrs![hrefIndex][1];

      // Внешние ссылки (http/https)
      if (href.startsWith('http://') || href.startsWith('https://')) {
        token.attrSet('class', 'external-link');
        token.attrSet('target', '_blank');
        token.attrSet('rel', 'noopener noreferrer');
      }
      // Внутренние ссылки на .md файлы
      else if (href.endsWith('.md')) {
        token.attrSet('class', 'internal-link');
        token.attrSet('data-target', href);
      }
    }

    return defaultRender(tokens, idx, options, env, self);
  };
}

// Создание и настройка markdown-it
export function createMarkdownRenderer(): MarkdownIt {
  const md = new MarkdownIt({
    html: false, // Безопасность: не разрешаем HTML
    linkify: true,
    typographer: true,
    highlight: (str: string, lang: string) => {
      if (lang && hljs.getLanguage(lang)) {
        try {
          return `<pre class="hljs"><code>${hljs.highlight(str, { language: lang }).value}</code></pre>`;
        } catch {}
      }
      return `<pre class="hljs"><code>${md.utils.escapeHtml(str)}</code></pre>`;
    }
  });

  // Включаем таблицы, strikethrough
  md.enable(['table', 'strikethrough']);

  // Подключаем кастомные плагины
  md.use(wikiLinksPlugin);
  md.use(customLinkRenderer);

  // Checkbox'ы в списках - простая замена через inline правило
  const originalListItemOpen = md.renderer.rules.list_item_open || function(tokens, idx, options, env, self) {
    return self.renderToken(tokens, idx, options);
  };

  md.renderer.rules.list_item_open = function(tokens, idx, options, env, self) {
    const token = tokens[idx];
    // Ищем следующий text token
    const nextTokenIdx = idx + 2;
    if (nextTokenIdx < tokens.length) {
      const textToken = tokens[nextTokenIdx];
      if (textToken.type === 'inline' && textToken.content) {
        // Проверяем на [ ] или [x]
        const uncheckedMatch = textToken.content.match(/^\[ \] (.+)/);
        const checkedMatch = textToken.content.match(/^\[x\] (.+)/i);

        if (uncheckedMatch) {
          textToken.content = `<input type="checkbox" disabled> ${uncheckedMatch[1]}`;
          token.attrSet('class', 'task-list-item');
        } else if (checkedMatch) {
          textToken.content = `<input type="checkbox" disabled checked> ${checkedMatch[1]}`;
          token.attrSet('class', 'task-list-item');
        }
      }
    }
    return originalListItemOpen(tokens, idx, options, env, self);
  };

  return md;
}

// Singleton instance
let mdInstance: MarkdownIt | null = null;

export function getMarkdownRenderer(): MarkdownIt {
  if (!mdInstance) {
    mdInstance = createMarkdownRenderer();
  }
  return mdInstance;
}

export function renderMarkdown(content: string): string {
  const md = getMarkdownRenderer();
  return md.render(content);
}
