import { keymap } from '@codemirror/view';
import type { EditorView } from '@codemirror/view';

export const markdownKeymap = keymap.of([
  {
    key: 'Enter',
    run(view: EditorView) {
      const selection = view.state.selection.main;
      const line = view.state.doc.lineAt(selection.head);
      const text = line.text;

      // Чекбоксы: - [ ] или - [x]
      const checkMatch = text.match(/^(\s*)- \[([ x])\]\s(.*)$/);
      if (checkMatch) {
        const indent = checkMatch[1];
        const content = checkMatch[3];

        // Если строка пустая (только чекбокс без текста) → выход из списка
        if (content.trim() === '') {
          view.dispatch({
            changes: { from: line.from, to: line.to, insert: '' },
            selection: { anchor: line.from },
          });
          return true;
        }

        // Иначе → новый элемент с чекбоксом
        view.dispatch({
          changes: {
            from: selection.head,
            insert: '\n' + indent + '- [ ] ',
          },
          selection: { anchor: selection.head + indent.length + 7 },
        });
        return true;
      }

      // Обычные буллет-списки: - или *
      const bulletMatch = text.match(/^(\s*)([-*])\s(.*)$/);
      if (bulletMatch) {
        const indent = bulletMatch[1];
        const marker = bulletMatch[2];
        const content = bulletMatch[3];

        // Если строка пустая (только маркер без текста) → выход из списка
        if (content.trim() === '') {
          view.dispatch({
            changes: { from: line.from, to: line.to, insert: '' },
            selection: { anchor: line.from },
          });
          return true;
        }

        // Иначе → новый элемент с тем же маркером
        view.dispatch({
          changes: {
            from: selection.head,
            insert: '\n' + indent + marker + ' ',
          },
          selection: { anchor: selection.head + indent.length + 3 },
        });
        return true;
      }

      // Нумерованные списки: 1. , 2. , и т.д.
      const numberedMatch = text.match(/^(\s*)(\d+)\.\s(.*)$/);
      if (numberedMatch) {
        const indent = numberedMatch[1];
        const num = parseInt(numberedMatch[2]);
        const content = numberedMatch[3];

        // Если строка пустая → выход из списка
        if (content.trim() === '') {
          view.dispatch({
            changes: { from: line.from, to: line.to, insert: '' },
            selection: { anchor: line.from },
          });
          return true;
        }

        // Иначе → новый элемент с увеличенным номером
        const nextNum = num + 1;
        view.dispatch({
          changes: {
            from: selection.head,
            insert: '\n' + indent + nextNum + '. ',
          },
          selection: {
            anchor: selection.head + indent.length + nextNum.toString().length + 3,
          },
        });
        return true;
      }

      return false; // default Enter
    },
  },
  {
    key: 'Tab',
    run(view: EditorView) {
      const selection = view.state.selection.main;
      const line = view.state.doc.lineAt(selection.head);
      const text = line.text;

      // Проверяем, является ли строка элементом списка
      const listMatch = text.match(/^(\s*)([-*]|\d+\.|- \[[ x]\])\s/);
      if (listMatch) {
        // Добавить отступ к элементу списка
        view.dispatch({
          changes: { from: line.from, insert: '  ' },
          selection: { anchor: selection.head + 2 },
        });
        return true;
      }

      return false;
    },
  },
  {
    key: 'Shift-Tab',
    run(view: EditorView) {
      const selection = view.state.selection.main;
      const line = view.state.doc.lineAt(selection.head);
      const text = line.text;

      // Проверяем, является ли строка элементом списка и есть ли отступ
      const listMatch = text.match(/^(\s*)([-*]|\d+\.|- \[[ x]\])\s/);
      if (listMatch && line.text.startsWith('  ')) {
        // Убрать отступ
        view.dispatch({
          changes: { from: line.from, to: line.from + 2, insert: '' },
          selection: { anchor: Math.max(line.from, selection.head - 2) },
        });
        return true;
      }

      return false;
    },
  },
]);
