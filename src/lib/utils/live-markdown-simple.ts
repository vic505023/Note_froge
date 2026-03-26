import { ViewPlugin, Decoration, DecorationSet, EditorView } from '@codemirror/view';
import { RangeSetBuilder } from '@codemirror/state';

function buildDecorations(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const cursor = view.state.selection.main.head;
  const cursorLine = view.state.doc.lineAt(cursor);

  for (const { from, to } of view.visibleRanges) {
    let pos = from;

    while (pos < to) {
      const line = view.state.doc.lineAt(pos);
      const text = line.text;
      const isActiveLine = line.number === cursorLine.number;

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

      // Списки: -
      const listMatch = text.match(/^(\s*)([-*])\s/);
      if (listMatch && !isActiveLine) {
        const start = line.from + listMatch[1].length;
        builder.add(
          start,
          start + 2,
          Decoration.mark({
            attributes: {
              style: 'font-size: 0; width: 0; overflow: hidden;'
            },
            class: 'cm-list-marker-hidden'
          })
        );
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
        const showMarkers = cursor >= startPos - 1 && cursor <= endPos + 1;

        if (!showMarkers) {
          decoSpecs.push({
            from: startPos,
            to: startPos + 2,
            decoration: Decoration.mark({
              attributes: { style: 'font-size: 0; width: 0; display: inline-block; overflow: hidden;' }
            })
          });
        }

        decoSpecs.push({
          from: startPos + 2,
          to: endPos - 2,
          decoration: Decoration.mark({
            attributes: { style: 'font-weight: 700;' }
          })
        });

        if (!showMarkers) {
          decoSpecs.push({
            from: endPos - 2,
            to: endPos,
            decoration: Decoration.mark({
              attributes: { style: 'font-size: 0; width: 0; display: inline-block; overflow: hidden;' }
            })
          });
        }
      }

      // `code`
      const codeRegex = /`([^`]+)`/g;
      while ((match = codeRegex.exec(text)) !== null) {
        const startPos = line.from + match.index;
        const endPos = startPos + match[0].length;
        const showMarkers = cursor >= startPos - 1 && cursor <= endPos + 1;

        if (!showMarkers) {
          decoSpecs.push({
            from: startPos,
            to: startPos + 1,
            decoration: Decoration.mark({
              attributes: { style: 'font-size: 0; width: 0; display: inline-block; overflow: hidden;' }
            })
          });
        }

        decoSpecs.push({
          from: startPos + 1,
          to: endPos - 1,
          decoration: Decoration.mark({
            attributes: { style: 'font-family: "JetBrains Mono", monospace; background: rgba(255,255,255,0.1); padding: 2px 4px; border-radius: 3px;' }
          })
        });

        if (!showMarkers) {
          decoSpecs.push({
            from: endPos - 1,
            to: endPos,
            decoration: Decoration.mark({
              attributes: { style: 'font-size: 0; width: 0; display: inline-block; overflow: hidden;' }
            })
          });
        }
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

export const liveMarkdownSimple = ViewPlugin.fromClass(
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
);
