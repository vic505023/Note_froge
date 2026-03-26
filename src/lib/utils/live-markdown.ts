import {
  ViewPlugin,
  Decoration,
  DecorationSet,
  EditorView,
  WidgetType,
} from '@codemirror/view';
import { RangeSetBuilder } from '@codemirror/state';
import type { EditorState } from '@codemirror/state';

// Widget для буллет-точки
class BulletWidget extends WidgetType {
  toDOM() {
    const span = document.createElement('span');
    span.className = 'cm-bullet';
    span.textContent = '•';
    return span;
  }
}

// Widget для чекбокса
class CheckboxWidget extends WidgetType {
  constructor(readonly checked: boolean, readonly pos: number) {
    super();
  }

  toDOM(view: EditorView) {
    const input = document.createElement('input');
    input.type = 'checkbox';
    input.checked = this.checked;
    input.className = 'cm-checkbox';
    input.onclick = (e) => {
      e.preventDefault();
      const line = view.state.doc.lineAt(this.pos);
      const text = line.text;
      const newText = this.checked
        ? text.replace(/- \[x\]/, '- [ ]')
        : text.replace(/- \[ \]/, '- [x]');

      view.dispatch({
        changes: { from: line.from, to: line.to, insert: newText },
      });
    };
    return input;
  }

  eq(other: CheckboxWidget) {
    return other.checked === this.checked && other.pos === this.pos;
  }

  ignoreEvent() {
    return false;
  }
}

// Widget для горизонтальной линии
class HorizontalRuleWidget extends WidgetType {
  toDOM() {
    const hr = document.createElement('div');
    hr.className = 'cm-hr';
    return hr;
  }
}

function buildDecorations(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const cursor = view.state.selection.main.head;
  const cursorLine = view.state.doc.lineAt(cursor);

  console.log('[LiveMarkdown] Building decorations, cursor at line:', cursorLine.number);

  for (const { from, to } of view.visibleRanges) {
    let pos = from;

    while (pos < to) {
      const line = view.state.doc.lineAt(pos);
      const lineText = line.text;
      const isActiveLine = line.number === cursorLine.number;

      // Не декорировать строку где стоит курсор
      if (!isActiveLine) {
        // Заголовки: ## Text → ПОЛНОСТЬЮ скрыть ##
        const headingMatch = lineText.match(/^(#{1,6})\s+(.*)$/);
        if (headingMatch) {
          const level = headingMatch[1].length;
          const hashLen = headingMatch[1].length;

          // ПОЛНОСТЬЮ СКРЫТЬ "## " (включая пробел)
          builder.add(
            line.from,
            line.from + hashLen + 1,
            Decoration.replace({})
          );

          // Стилизовать текст заголовка
          builder.add(
            line.from + hashLen + 1,
            line.to,
            Decoration.mark({ class: `cm-heading cm-heading-${level}` })
          );
        }

        // Горизонтальная линия: ---, ***, ___
        if (/^(\-{3,}|\*{3,}|_{3,})$/.test(lineText.trim())) {
          builder.add(
            line.from,
            line.to,
            Decoration.replace({
              widget: new HorizontalRuleWidget(),
            })
          );
        }

        // Чекбоксы: - [ ] или - [x]
        const checkboxMatch = lineText.match(/^(\s*)- \[([ x])\]\s/);
        if (checkboxMatch) {
          const checked = checkboxMatch[2] === 'x';
          const checkboxStart = line.from + checkboxMatch[1].length;
          const checkboxEnd = checkboxStart + 6; // "- [x] " или "- [ ] "

          builder.add(
            checkboxStart,
            checkboxEnd,
            Decoration.replace({
              widget: new CheckboxWidget(checked, line.from),
            })
          );
        }
        // Обычные буллет-списки: - или *
        else {
          const bulletMatch = lineText.match(/^(\s*)([-*])\s/);
          if (bulletMatch) {
            const bulletStart = line.from + bulletMatch[1].length;
            const bulletEnd = bulletStart + 2; // "- " или "* "

            builder.add(
              bulletStart,
              bulletEnd,
              Decoration.replace({
                widget: new BulletWidget(),
              })
            );
          }
        }

        // Цитаты: > text → ПОЛНОСТЬЮ скрыть >
        const quoteMatch = lineText.match(/^(>\s+)(.*)/);
        if (quoteMatch) {
          builder.add(
            line.from,
            line.from + quoteMatch[1].length,
            Decoration.replace({})
          );
          // Стилизовать текст цитаты
          builder.add(
            line.from + quoteMatch[1].length,
            line.to,
            Decoration.mark({ class: 'cm-blockquote' })
          );
        }

        // Inline форматирование (только если не внутри кода-блока)
        if (!isInsideCodeBlock(line.from, view.state)) {
          decorateInlineFormatting(line, builder, view.state, cursor);
        }
      }

      pos = line.to + 1;
    }
  }

  return builder.finish();
}

function decorateInlineFormatting(
  line: { from: number; to: number; text: string },
  builder: RangeSetBuilder<Decoration>,
  state: EditorState,
  cursor: number
) {
  const text = line.text;

  // Жирный текст: **text** → ПОЛНОСТЬЮ скрыть **
  const boldRegex = /\*\*([^\*]+)\*\*/g;
  let match: RegExpExecArray | null;
  while ((match = boldRegex.exec(text)) !== null) {
    const startPos = line.from + match.index;
    const endPos = startPos + match[0].length;

    // НЕ декорировать если курсор внутри (между открывающими и закрывающими **)
    if (cursor >= startPos && cursor <= endPos) {
      continue;
    }

    // ПОЛНОСТЬЮ СКРЫТЬ открывающие **
    builder.add(
      startPos,
      startPos + 2,
      Decoration.replace({})
    );

    // Сделать текст жирным
    builder.add(
      startPos + 2,
      endPos - 2,
      Decoration.mark({ class: 'cm-bold' })
    );

    // ПОЛНОСТЬЮ СКРЫТЬ закрывающие **
    builder.add(
      endPos - 2,
      endPos,
      Decoration.replace({})
    );
  }

  // Курсив: *text* (но не **text**)
  const italicRegex = /(?<!\*)\*(?!\*)([^\*]+)\*(?!\*)/g;
  while ((match = italicRegex.exec(text)) !== null) {
    const startPos = line.from + match.index;
    const endPos = startPos + match[0].length;

    if (cursor >= startPos && cursor <= endPos) {
      continue;
    }

    builder.add(
      startPos,
      startPos + 1,
      Decoration.replace({})
    );

    builder.add(
      startPos + 1,
      endPos - 1,
      Decoration.mark({ class: 'cm-italic' })
    );

    builder.add(
      endPos - 1,
      endPos,
      Decoration.replace({})
    );
  }

  // Зачеркнутый: ~~text~~
  const strikeRegex = /~~([^~]+)~~/g;
  while ((match = strikeRegex.exec(text)) !== null) {
    const startPos = line.from + match.index;
    const endPos = startPos + match[0].length;

    if (cursor >= startPos && cursor <= endPos) {
      continue;
    }

    builder.add(
      startPos,
      startPos + 2,
      Decoration.replace({})
    );

    builder.add(
      startPos + 2,
      endPos - 2,
      Decoration.mark({ class: 'cm-strikethrough' })
    );

    builder.add(
      endPos - 2,
      endPos,
      Decoration.replace({})
    );
  }

  // Выделение: ==text==
  const highlightRegex = /==([^=]+)==/g;
  while ((match = highlightRegex.exec(text)) !== null) {
    const startPos = line.from + match.index;
    const endPos = startPos + match[0].length;

    if (cursor >= startPos && cursor <= endPos) {
      continue;
    }

    builder.add(
      startPos,
      startPos + 2,
      Decoration.replace({})
    );

    builder.add(
      startPos + 2,
      endPos - 2,
      Decoration.mark({ class: 'cm-highlight' })
    );

    builder.add(
      endPos - 2,
      endPos,
      Decoration.replace({})
    );
  }

  // Inline код: `code`
  const codeRegex = /`([^`]+)`/g;
  while ((match = codeRegex.exec(text)) !== null) {
    const startPos = line.from + match.index;
    const endPos = startPos + match[0].length;

    if (cursor >= startPos && cursor <= endPos) {
      continue;
    }

    builder.add(
      startPos,
      startPos + 1,
      Decoration.replace({})
    );

    builder.add(
      startPos + 1,
      endPos - 1,
      Decoration.mark({ class: 'cm-inline-code' })
    );

    builder.add(
      endPos - 1,
      endPos,
      Decoration.replace({})
    );
  }

  // Ссылки: [text](url) → ПОЛНОСТЬЮ скрыть скобки и URL
  const linkRegex = /\[([^\]]+)\]\(([^\)]+)\)/g;
  while ((match = linkRegex.exec(text)) !== null) {
    const startPos = line.from + match.index;
    const textStart = startPos + 1;
    const textEnd = textStart + match[1].length;
    const urlStart = textEnd + 2;
    const urlEnd = urlStart + match[2].length;
    const endPos = urlEnd + 1;

    if (cursor >= startPos && cursor <= endPos) {
      continue;
    }

    // ПОЛНОСТЬЮ СКРЫТЬ "["
    builder.add(
      startPos,
      textStart,
      Decoration.replace({})
    );

    // Стилизовать текст ссылки
    builder.add(
      textStart,
      textEnd,
      Decoration.mark({ class: 'cm-link-text' })
    );

    // ПОЛНОСТЬЮ СКРЫТЬ "](" + URL + ")"
    builder.add(
      textEnd,
      endPos,
      Decoration.replace({})
    );
  }
}

function isInsideCodeBlock(pos: number, state: EditorState): boolean {
  const line = state.doc.lineAt(pos);
  let insideBlock = false;

  // Проверяем все строки до текущей
  for (let i = 1; i < line.number; i++) {
    const checkLine = state.doc.line(i);
    if (checkLine.text.trim().startsWith('```')) {
      insideBlock = !insideBlock;
    }
  }

  return insideBlock;
}

export const liveMarkdown = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = buildDecorations(view);
    }

    update(update: any) {
      if (
        update.docChanged ||
        update.viewportChanged ||
        update.selectionSet
      ) {
        this.decorations = buildDecorations(update.view);
      }
    }
  },
  {
    decorations: (v) => v.decorations,
  }
);
