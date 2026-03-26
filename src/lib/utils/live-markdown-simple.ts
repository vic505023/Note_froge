import { ViewPlugin, Decoration, DecorationSet, EditorView, WidgetType } from '@codemirror/view';
import { RangeSetBuilder, Extension } from '@codemirror/state';

// Widget для буллета (точки) в списках
class BulletWidget extends WidgetType {
  toDOM() {
    const span = document.createElement('span');
    span.className = 'cm-list-bullet';
    span.textContent = '•';
    return span;
  }

  ignoreEvent() {
    return false;
  }
}

// Widget для чекбокса
class CheckboxWidget extends WidgetType {
  constructor(readonly checked: boolean, readonly lineFrom: number, readonly view: EditorView) {
    super();
  }

  toDOM() {
    const wrapper = document.createElement('span');
    wrapper.className = 'cm-task-checkbox-wrapper';

    const checkbox = document.createElement('input');
    checkbox.type = 'checkbox';
    checkbox.checked = this.checked;
    checkbox.className = 'cm-task-checkbox';

    // Обработчик клика для переключения состояния
    checkbox.addEventListener('mousedown', (e) => {
      e.preventDefault();
      this.toggleCheckbox();
    });

    wrapper.appendChild(checkbox);
    return wrapper;
  }

  toggleCheckbox() {
    const line = this.view.state.doc.lineAt(this.lineFrom);
    const text = line.text;

    // Находим [ ] или [x] и меняем состояние
    let newText: string;
    if (text.match(/^(\s*)- \[ \] /)) {
      newText = text.replace(/^(\s*)- \[ \] /, '$1- [x] ');
    } else if (text.match(/^(\s*)- \[x\] /i)) {
      newText = text.replace(/^(\s*)- \[x\] /i, '$1- [ ] ');
    } else {
      return;
    }

    // Применяем изменение
    this.view.dispatch({
      changes: {
        from: line.from,
        to: line.to,
        insert: newText
      }
    });
  }

  ignoreEvent() {
    return false;
  }
}

function buildDecorations(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const cursor = view.state.selection.main.head;
  const cursorLine = view.state.doc.lineAt(cursor);

  // Проверяем, идет ли выделение текста (чтобы избежать дребезга)
  const selection = view.state.selection.main;
  const isSelecting = selection.anchor !== selection.head;

  for (const { from, to } of view.visibleRanges) {
    // ПРОХОД 1: Собираем информацию о блоках кода
    const codeBlocks: Array<{ start: number; end: number }> = [];
    let inCodeBlock = false;
    let blockStartLine = 0;

    // Проверяем состояние до visible range
    const startLine = view.state.doc.lineAt(from);
    for (let lineNum = 1; lineNum < startLine.number; lineNum++) {
      const checkLine = view.state.doc.line(lineNum);
      if (checkLine.text.trim().startsWith('```')) {
        if (!inCodeBlock) {
          blockStartLine = lineNum;
          inCodeBlock = true;
        } else {
          codeBlocks.push({ start: blockStartLine, end: lineNum });
          inCodeBlock = false;
        }
      }
    }

    // Собираем блоки в visible range
    let pos = from;
    while (pos < to) {
      const line = view.state.doc.lineAt(pos);
      if (line.text.trim().startsWith('```')) {
        if (!inCodeBlock) {
          blockStartLine = line.number;
          inCodeBlock = true;
        } else {
          codeBlocks.push({ start: blockStartLine, end: line.number });
          inCodeBlock = false;
        }
      }
      pos = line.to + 1;
    }

    // Проверяем, находится ли курсор внутри какого-либо блока кода
    const cursorInCodeBlock = codeBlocks.find(block =>
      cursorLine.number >= block.start && cursorLine.number <= block.end
    );

    // ПРОХОД 2: Применяем декорации
    pos = from;
    inCodeBlock = false;
    for (let lineNum = 1; lineNum < startLine.number; lineNum++) {
      const checkLine = view.state.doc.line(lineNum);
      if (checkLine.text.trim().startsWith('```')) {
        inCodeBlock = !inCodeBlock;
      }
    }

    while (pos < to) {
      const line = view.state.doc.lineAt(pos);
      const text = line.text;
      const isActiveLine = line.number === cursorLine.number;

      // Проверяем начало/конец блока кода: ```
      if (text.trim().startsWith('```')) {
        const wasInCodeBlock = inCodeBlock;
        inCodeBlock = !inCodeBlock;

        console.log('[Live MD] Code block fence at line', line.number,
                    'text:', text, 'was inside:', wasInCodeBlock, 'now inside:', inCodeBlock,
                    'cursorInCodeBlock:', !!cursorInCodeBlock);

        // Добавляем фон блока к строке с ```
        builder.add(
          line.from,
          line.from,
          Decoration.line({
            class: 'cm-code-block-line'
          })
        );

        // Скрываем текст ``` если курсор НЕ внутри блока
        if (!cursorInCodeBlock) {
          builder.add(
            line.from,
            line.to,
            Decoration.replace({})
          );
        }

        pos = line.to + 1;
        continue;
      }

      // Если внутри блока кода, применяем форматирование кода
      if (inCodeBlock) {
        console.log('[Live MD] Formatting code block line', line.number, ':', text.substring(0, 20));

        // Line decoration - применяется ко всей строке как единице
        builder.add(
          line.from,
          line.from,
          Decoration.line({
            class: 'cm-code-block-line'
          })
        );

        // Mark decoration для текста - моноширинный шрифт
        builder.add(
          line.from,
          line.to,
          Decoration.mark({
            class: 'cm-code-block-text'
          })
        );

        pos = line.to + 1;
        continue;
      }

      // Заголовки: ##
      const headingMatch = text.match(/^(#{1,6})\s+(.*)$/);
      if (headingMatch) {
        const level = headingMatch[1].length;
        const hashLen = headingMatch[1].length + 1;

        if (!isActiveLine) {
          builder.add(
            line.from,
            line.from + hashLen,
            Decoration.mark({
              attributes: {
                style: 'font-size: 0; width: 0; display: inline-block; overflow: hidden;'
              }
            })
          );
        }

        const sizes = ['1.8em', '1.5em', '1.3em', '1.1em', '1.05em', '1em'];
        builder.add(
          line.from + hashLen,
          line.to,
          Decoration.mark({
            attributes: {
              style: `font-size: ${sizes[level - 1]}; font-weight: 600; line-height: 1.4;`
            }
          })
        );

        pos = line.to + 1;
        continue;
      }

      // Горизонтальная линия: ---
      const hrMatch = /^(\-{3,}|\*{3,}|_{3,})$/.test(text.trim());
      if (hrMatch) {
        if (!isActiveLine) {
          builder.add(
            line.from,
            line.to,
            Decoration.mark({
              class: 'cm-hr-hidden'
            })
          );
        }
        pos = line.to + 1;
        continue;
      }

      // Task list items: - [ ] или - [x]
      const taskMatch = text.match(/^(\s*)- \[([ x])\] /i);
      if (taskMatch) {
        const indent = taskMatch[1].length;
        const checked = taskMatch[2].toLowerCase() === 'x';
        const markerStart = line.from + indent;
        const markerEnd = markerStart + 6; // "- [ ] " или "- [x] "

        // Проверяем: курсор НЕ в области маркера (заменяем маркер на checkbox)
        // Маркер показывается только когда курсор находится между началом и концом маркера
        const cursorInMarker = isActiveLine && cursor >= markerStart && cursor <= markerEnd;

        if (!cursorInMarker) {
          // Заменяем весь маркер "- [ ] " на checkbox widget
          builder.add(
            markerStart,
            markerEnd,
            Decoration.replace({
              widget: new CheckboxWidget(checked, line.from, view)
            })
          );
        }

        pos = line.to + 1;
        continue;
      }

      // Обычные списки: -
      const listMatch = text.match(/^(\s*)([-*])\s/);
      if (listMatch) {
        const indent = listMatch[1].length;
        const markerStart = line.from + indent;
        const markerEnd = markerStart + 2; // "- " или "* "

        // Проверяем: курсор НЕ в области маркера (заменяем маркер на bullet)
        // Маркер показывается только когда курсор находится между началом и концом маркера
        const cursorInMarker = isActiveLine && cursor >= markerStart && cursor <= markerEnd;

        if (!cursorInMarker) {
          // Заменяем весь маркер "- " на bullet widget
          builder.add(
            markerStart,
            markerEnd,
            Decoration.replace({
              widget: new BulletWidget()
            })
          );
        }
      }

      type DecoSpec = {
        from: number;
        to: number;
        decoration: Decoration;
      };
      const decoSpecs: DecoSpec[] = [];
      let match: RegExpExecArray | null;

      // **bold**
      const boldRegex = /\*\*([^\*]+)\*\*/g;
      while ((match = boldRegex.exec(text)) !== null) {
        const startPos = line.from + match.index;
        const endPos = startPos + match[0].length;
        // Показываем маркеры если: идет выделение ИЛИ курсор в области форматирования
        const showMarkers = isSelecting || (cursor >= startPos && cursor <= endPos);

        if (!showMarkers) {
          // Используем replace чтобы полностью удалить маркеры
          decoSpecs.push({
            from: startPos,
            to: startPos + 2,
            decoration: Decoration.replace({})
          });

          decoSpecs.push({
            from: endPos - 2,
            to: endPos,
            decoration: Decoration.replace({})
          });
        }

        // Делаем текст жирным
        decoSpecs.push({
          from: startPos + 2,
          to: endPos - 2,
          decoration: Decoration.mark({
            attributes: { style: 'font-weight: 700;' }
          })
        });
      }

      // `code`
      const codeRegex = /`([^`]+)`/g;
      while ((match = codeRegex.exec(text)) !== null) {
        const startPos = line.from + match.index;
        const endPos = startPos + match[0].length;
        // Показываем маркеры если: идет выделение ИЛИ курсор в области форматирования
        const showMarkers = isSelecting || (cursor >= startPos && cursor <= endPos);

        if (!showMarkers) {
          // Используем replace чтобы полностью удалить маркеры
          decoSpecs.push({
            from: startPos,
            to: startPos + 1,
            decoration: Decoration.replace({})
          });

          decoSpecs.push({
            from: endPos - 1,
            to: endPos,
            decoration: Decoration.replace({})
          });
        }

        // Форматируем код
        decoSpecs.push({
          from: startPos + 1,
          to: endPos - 1,
          decoration: Decoration.mark({
            attributes: { style: 'font-family: "JetBrains Mono", monospace; background: rgba(255,255,255,0.1); padding: 2px 4px; border-radius: 3px;' }
          })
        });
      }

      decoSpecs.sort((a, b) => a.from - b.from);
      for (const spec of decoSpecs) {
        builder.add(spec.from, spec.to, spec.decoration);
      }

      pos = line.to + 1;
    }
  }

  return builder.finish();
}

// Обработчик клика для корректировки позиции курсора
// Предотвращает попадание курсора в скрытые символы форматирования
const clickHandler = EditorView.updateListener.of((update) => {
  // Проверяем что selection изменился (клик или движение курсора)
  if (!update.selectionSet || update.docChanged) {
    return;
  }

  const pos = update.state.selection.main.head;
  const line = update.state.doc.lineAt(pos);
  const text = line.text;
  const offset = pos - line.from;

  console.log('[Live MD] Selection at offset:', offset, 'in line:', text);

  let needsCorrection = false;
  let newPos = pos;

  // Проверяем bold: **текст**
  const boldRegex = /\*\*([^\*]+)\*\*/g;
  let match: RegExpExecArray | null;

  while ((match = boldRegex.exec(text)) !== null) {
    const matchStart = match.index;
    const matchEnd = matchStart + match[0].length;
    const contentStart = matchStart + 2;
    const contentEnd = matchEnd - 2;

    console.log('[Live MD] Found bold:', match[0], 'at', matchStart, '-', matchEnd,
                'content:', contentStart, '-', contentEnd, 'cursor offset:', offset);

    // Курсор попал на открывающие ** → сдвигаем после них
    if (offset > matchStart && offset < contentStart) {
      console.log('[Live MD] Cursor on opening **, correcting to', contentStart);
      newPos = line.from + contentStart;
      needsCorrection = true;
      break;
    }

    // Курсор попал внутри закрывающих ** → сдвигаем за них
    if (offset > contentEnd && offset <= matchEnd) {
      console.log('[Live MD] Cursor inside closing **, correcting to', matchEnd);
      newPos = line.from + matchEnd;
      needsCorrection = true;
      break;
    }
  }

  // Проверяем code: `текст`
  if (!needsCorrection) {
    const codeRegex = /`([^`]+)`/g;
    while ((match = codeRegex.exec(text)) !== null) {
      const matchStart = match.index;
      const matchEnd = matchStart + match[0].length;
      const contentStart = matchStart + 1;
      const contentEnd = matchEnd - 1;

      console.log('[Live MD] Found code:', match[0], 'at', matchStart, '-', matchEnd);

      // Курсор попал на открывающий ` → сдвигаем после него
      if (offset > matchStart && offset < contentStart) {
        console.log('[Live MD] Cursor on opening `, correcting to', contentStart);
        newPos = line.from + contentStart;
        needsCorrection = true;
        break;
      }

      // Курсор попал внутри закрывающего ` → сдвигаем за него
      if (offset > contentEnd && offset <= matchEnd) {
        console.log('[Live MD] Cursor inside closing `, correcting to', matchEnd);
        newPos = line.from + matchEnd;
        needsCorrection = true;
        break;
      }
    }
  }

  if (needsCorrection && newPos !== pos) {
    console.log('[Live MD] Correcting cursor: offset', offset, '→', newPos - line.from, 'in text:', text);
    // Используем requestAnimationFrame чтобы dispatch выполнился после завершения текущего update
    requestAnimationFrame(() => {
      if (update.view.state.selection.main.head === pos) {
        update.view.dispatch({
          selection: { anchor: newPos }
        });
        console.log('[Live MD] Cursor corrected');
      } else {
        console.log('[Live MD] Cursor already moved, skipping correction');
      }
    });
  }
});

export const liveMarkdownSimple: Extension = [
  ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = buildDecorations(view);
      }

      update(update: any) {
        if (update.docChanged || update.viewportChanged || update.selectionSet) {
          this.decorations = buildDecorations(update.view);
        }
      }
    },
    { decorations: (v) => v.decorations }
  ),
  clickHandler
];
