<script lang="ts">
  import type { FileNode } from '../types';

  interface Props {
    x: number;
    y: number;
    node: FileNode;
    onCopy: (node: FileNode) => void;
    onCut: (node: FileNode) => void;
    onPaste: (node: FileNode) => void;
    onDelete: (node: FileNode) => void;
    onRename: (node: FileNode) => void;
    onClose: () => void;
    canPaste: boolean;
  }

  let { x, y, node, onCopy, onCut, onPaste, onDelete, onRename, onClose, canPaste }: Props = $props();

  function handleCopy() {
    onCopy(node);
    onClose();
  }

  function handleCut() {
    onCut(node);
    onClose();
  }

  function handlePaste() {
    onPaste(node);
    onClose();
  }

  function handleDelete() {
    onDelete(node);
    onClose();
  }

  function handleRename() {
    onRename(node);
    onClose();
  }

  // Close on click outside
  function handleClickOutside(e: MouseEvent) {
    if (!(e.target as HTMLElement).closest('.context-menu')) {
      onClose();
    }
  }
</script>

<svelte:window onclick={handleClickOutside} />

<div
  class="context-menu"
  style="left: {x}px; top: {y}px"
>
  <button class="menu-item" onclick={handleCopy}>
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <rect x="2" y="2" width="8" height="8" rx="1" stroke="currentColor" stroke-width="1.2" fill="none"/>
      <path d="M4 4V3C4 2.44772 4.44772 2 5 2H11C11.5523 2 12 2.44772 12 3V9C12 9.55228 11.5523 10 11 10H10" stroke="currentColor" stroke-width="1.2" fill="none"/>
    </svg>
    <span>Copy</span>
    <span class="shortcut">Ctrl+C</span>
  </button>

  <button class="menu-item" onclick={handleCut}>
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path d="M3 3L11 11M11 3L3 11" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
    </svg>
    <span>Cut</span>
    <span class="shortcut">Ctrl+X</span>
  </button>

  <button class="menu-item" onclick={handlePaste} disabled={!canPaste}>
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <rect x="3" y="4" width="8" height="8" rx="1" stroke="currentColor" stroke-width="1.2" fill="none"/>
      <path d="M5 4V3C5 2.44772 5.44772 2 6 2H8C8.55228 2 9 2.44772 9 3V4" stroke="currentColor" stroke-width="1.2"/>
    </svg>
    <span>Paste</span>
    <span class="shortcut">Ctrl+V</span>
  </button>

  <div class="menu-separator"></div>

  <button class="menu-item" onclick={handleRename}>
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path d="M10 1L13 4L5 12H2V9L10 1Z" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
    </svg>
    <span>Rename</span>
    <span class="shortcut">F2</span>
  </button>

  <button class="menu-item danger" onclick={handleDelete}>
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path d="M3 4H11M5 4V3H9V4M5 7V10M9 7V10M4 4L4.5 11H9.5L10 4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
    </svg>
    <span>Delete</span>
    <span class="shortcut">Del</span>
  </button>
</div>

<style>
  .context-menu {
    position: fixed;
    background: rgba(26, 27, 38, 0.7);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 4px;
    min-width: 200px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    z-index: 9999;
  }

  .menu-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 0.875rem;
    text-align: left;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .menu-item:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .menu-item:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .menu-item.danger {
    color: var(--error);
  }

  .menu-item.danger:hover:not(:disabled) {
    background: rgba(247, 118, 142, 0.1);
  }

  .menu-item svg {
    flex-shrink: 0;
  }

  .menu-item span:first-of-type {
    flex: 1;
  }

  .shortcut {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-family: 'JetBrains Mono', monospace;
  }

  .menu-separator {
    height: 1px;
    background: var(--border-soft);
    margin: 4px 0;
  }
</style>
