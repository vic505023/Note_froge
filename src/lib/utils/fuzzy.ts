export interface FuzzyMatch {
  score: number;
  indices: number[];
}

/**
 * Fuzzy matching algorithm
 * Returns match score and indices of matched characters
 * Lower score is better
 */
export function fuzzyMatch(query: string, target: string): FuzzyMatch | null {
  if (!query) return { score: 0, indices: [] };

  const queryLower = query.toLowerCase();
  const targetLower = target.toLowerCase();

  let score = 0;
  let queryIndex = 0;
  const indices: number[] = [];

  // Check if all query characters are in target (in order)
  for (let targetIndex = 0; targetIndex < targetLower.length; targetIndex++) {
    if (queryIndex >= queryLower.length) break;

    if (targetLower[targetIndex] === queryLower[queryIndex]) {
      indices.push(targetIndex);

      // Score based on position (earlier is better)
      score += targetIndex;

      // Bonus for consecutive matches
      if (indices.length > 1 && indices[indices.length - 1] === indices[indices.length - 2] + 1) {
        score -= 5; // Consecutive match bonus
      }

      // Bonus for matching at word boundaries
      if (targetIndex === 0 || /[^a-zA-Z0-9]/.test(targetLower[targetIndex - 1])) {
        score -= 10; // Word boundary bonus
      }

      queryIndex++;
    }
  }

  // All query characters must be found
  if (queryIndex < queryLower.length) {
    return null;
  }

  // Penalty for length difference
  score += (targetLower.length - queryLower.length) * 2;

  return { score, indices };
}

/**
 * Fuzzy search through an array of strings
 * Returns sorted array of matches with their scores
 */
export function fuzzySearch<T>(
  query: string,
  items: T[],
  getText: (item: T) => string,
  limit = 15
): Array<{ item: T; match: FuzzyMatch }> {
  const matches: Array<{ item: T; match: FuzzyMatch }> = [];

  for (const item of items) {
    const text = getText(item);
    const match = fuzzyMatch(query, text);

    if (match) {
      matches.push({ item, match });
    }
  }

  // Sort by score (lower is better)
  matches.sort((a, b) => a.match.score - b.match.score);

  return matches.slice(0, limit);
}
