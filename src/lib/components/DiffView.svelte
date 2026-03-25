<script lang="ts">
  import { computeDiff, type DiffLine } from '../utils/diff';

  interface Props {
    oldText: string;
    newText: string;
    onApply: () => void;
    onReject: () => void;
  }

  let { oldText, newText, onApply, onReject }: Props = $props();

  let diff = $derived(computeDiff(oldText, newText));
</script>

<div class="diff-container">
  <div class="diff-header">
    <span class="diff-title">Proposed Changes</span>
    <div class="diff-actions">
      <button class="btn btn-secondary" onclick={onReject}>
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M2 2L12 12M12 2L2 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        Reject
      </button>
      <button class="btn btn-primary" onclick={onApply}>
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M2 7L5.5 10.5L12 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        Apply
      </button>
    </div>
  </div>

  <div class="diff-content">
    {#each diff as line}
      <div class="diff-line" class:added={line.type === 'added'} class:removed={line.type === 'removed'}>
        <span class="line-number">
          {#if line.lineNumber.old !== undefined}
            <span class="old-num">{line.lineNumber.old}</span>
          {:else}
            <span class="old-num empty"></span>
          {/if}
          {#if line.lineNumber.new !== undefined}
            <span class="new-num">{line.lineNumber.new}</span>
          {:else}
            <span class="new-num empty"></span>
          {/if}
        </span>
        <span class="line-content">{line.content}</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .diff-container {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
    margin: 12px 0;
    background: var(--bg-secondary);
  }

  .diff-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border);
  }

  .diff-title {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .diff-actions {
    display: flex;
    gap: 8px;
  }

  .btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border: none;
    border-radius: 4px;
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-primary {
    background: var(--accent);
    color: var(--bg-primary);
  }

  .btn-primary:hover {
    background: var(--accent-hover);
  }

  .btn-secondary {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .btn-secondary:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .diff-content {
    max-height: 400px;
    overflow-y: auto;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.8125rem;
    line-height: 1.5;
  }

  .diff-line {
    display: flex;
    align-items: flex-start;
  }

  .diff-line.added {
    background: rgba(158, 206, 106, 0.15);
  }

  .diff-line.removed {
    background: rgba(247, 118, 142, 0.15);
  }

  .line-number {
    display: flex;
    gap: 8px;
    padding: 2px 8px;
    min-width: 80px;
    color: var(--text-muted);
    user-select: none;
    border-right: 1px solid var(--border);
  }

  .old-num, .new-num {
    display: inline-block;
    width: 30px;
    text-align: right;
  }

  .old-num.empty, .new-num.empty {
    visibility: hidden;
  }

  .line-content {
    flex: 1;
    padding: 2px 12px;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .diff-line.added .line-content {
    color: var(--success);
  }

  .diff-line.removed .line-content {
    color: var(--error);
  }
</style>
