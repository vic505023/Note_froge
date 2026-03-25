export interface DiffLine {
  type: 'added' | 'removed' | 'unchanged';
  content: string;
  lineNumber: { old?: number; new?: number };
}

export function computeDiff(oldText: string, newText: string): DiffLine[] {
  const oldLines = oldText.split('\n');
  const newLines = newText.split('\n');

  const result: DiffLine[] = [];

  // Simple line-by-line comparison (LCS-based diff would be more accurate but more complex)
  // This is a basic implementation sufficient for MVP

  let oldIndex = 0;
  let newIndex = 0;

  while (oldIndex < oldLines.length || newIndex < newLines.length) {
    if (oldIndex >= oldLines.length) {
      // Remaining lines are all added
      result.push({
        type: 'added',
        content: newLines[newIndex],
        lineNumber: { new: newIndex + 1 }
      });
      newIndex++;
    } else if (newIndex >= newLines.length) {
      // Remaining lines are all removed
      result.push({
        type: 'removed',
        content: oldLines[oldIndex],
        lineNumber: { old: oldIndex + 1 }
      });
      oldIndex++;
    } else if (oldLines[oldIndex] === newLines[newIndex]) {
      // Lines match
      result.push({
        type: 'unchanged',
        content: oldLines[oldIndex],
        lineNumber: { old: oldIndex + 1, new: newIndex + 1 }
      });
      oldIndex++;
      newIndex++;
    } else {
      // Lines differ - check if it's a simple change or insertion/deletion
      // Look ahead to see if we can find a match
      let foundInNew = newLines.indexOf(oldLines[oldIndex], newIndex + 1);
      let foundInOld = oldLines.indexOf(newLines[newIndex], oldIndex + 1);

      if (foundInNew !== -1 && (foundInOld === -1 || foundInNew - newIndex < foundInOld - oldIndex)) {
        // Line was removed from old, and appears later in new
        // So current new lines are additions
        result.push({
          type: 'added',
          content: newLines[newIndex],
          lineNumber: { new: newIndex + 1 }
        });
        newIndex++;
      } else if (foundInOld !== -1) {
        // Line was added to new, and appears later in old
        // So current old line is a deletion
        result.push({
          type: 'removed',
          content: oldLines[oldIndex],
          lineNumber: { old: oldIndex + 1 }
        });
        oldIndex++;
      } else {
        // Lines are simply different (modification)
        // Show as remove + add
        result.push({
          type: 'removed',
          content: oldLines[oldIndex],
          lineNumber: { old: oldIndex + 1 }
        });
        result.push({
          type: 'added',
          content: newLines[newIndex],
          lineNumber: { new: newIndex + 1 }
        });
        oldIndex++;
        newIndex++;
      }
    }
  }

  return result;
}
