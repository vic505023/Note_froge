export class TimeoutError extends Error {
  override name = 'TimeoutError';
}

export function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number,
  message = `Timed out after ${timeoutMs}ms`
): Promise<T> {
  let timeoutId: ReturnType<typeof setTimeout> | undefined;

  const timeoutPromise = new Promise<never>((_, reject) => {
    timeoutId = setTimeout(() => reject(new TimeoutError(message)), timeoutMs);
  });

  return Promise.race([promise, timeoutPromise]).finally(() => {
    if (timeoutId !== undefined) clearTimeout(timeoutId);
  });
}

