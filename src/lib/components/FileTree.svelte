<script lang="ts">
  import type { FileNode } from '../types';
  import { notesStore } from '../stores/notes.svelte';
  import { clipboardStore } from '../stores/clipboard';
  import FileTree from './FileTree.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { confirm } from '@tauri-apps/plugin-dialog';

  interface Props {
    nodes: FileNode[];
    level?: number;
  }

  let { nodes, level = 0 }: Props = $props();

  let expandedDirs = $state<Set<string>>(new Set());
  let draggedNode = $state<FileNode | null>(null);
  let dropTargetPath = $state<string | null>(null);
  let contextMenu = $state<{ x: number; y: number; node: FileNode } | null>(null);
  let clipboard = $derived(clipboardStore.getState());

  function toggleDir(path: string) {
    if (expandedDirs.has(path)) {
      expandedDirs.delete(path);
    } else {
      expandedDirs.add(path);
    }
    expandedDirs = new Set(expandedDirs);
  }

  async function handleFileClick(node: FileNode) {
    console.log('File clicked:', node.path, 'is_dir:', node.is_dir);
    if (node.is_dir) {
      toggleDir(node.path);
    } else {
      await notesStore.openFile(node.path);
    }
  }

  function isActive(path: string) {
    return notesStore.currentFile === path;
  }

  // Drag & Drop handlers
  function handleDragStart(e: DragEvent, node: FileNode) {
    if (!e.dataTransfer) return;
    draggedNode = node;
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', node.path);
  }

  function handleDragOver(e: DragEvent, node: FileNode) {
    if (!draggedNode || !node.is_dir) return;

    // Check if valid drop target
    if (!canDropOn(draggedNode, node)) return;

    e.preventDefault();
    e.dataTransfer!.dropEffect = 'move';
    dropTargetPath = node.path;
  }

  function handleDragOverRoot(e: DragEvent) {
    if (level !== 0) return;
    if (!draggedNode) return;
    e.preventDefault();
    e.dataTransfer!.dropEffect = 'move';
    dropTargetPath = '';
  }

  function handleDragLeave() {
    dropTargetPath = null;
  }

  async function handleDrop(e: DragEvent, targetNode: FileNode) {
    e.preventDefault();
    if (!draggedNode || !targetNode.is_dir) return;

    await performMove(draggedNode, targetNode.path);
    cleanupDrag();
  }

  async function handleDropRoot(e: DragEvent) {
    if (level !== 0) return;
    e.preventDefault();
    if (!draggedNode) return;

    await performMove(draggedNode, '');
    cleanupDrag();
  }

  function canDropOn(dragged: FileNode, target: FileNode): boolean {
    // Can't drop on itself
    if (dragged.path === target.path) return false;

    // Can't drop folder into its own subdirectory
    if (dragged.is_dir && target.path.startsWith(dragged.path + '/')) return false;

    return true;
  }

  async function performMove(node: FileNode, targetDir: string) {
    try {
      const command = node.is_dir ? 'move_folder' : 'move_file';
      const newPath = await invoke<string>(command, {
        source: node.path,
        targetDir: targetDir,
      });

      // Update current file path if it was moved
      if (notesStore.currentFile === node.path) {
        notesStore.updateCurrentFilePath(newPath);
      }

      // Reload file tree
      await notesStore.loadFiles();
    } catch (err) {
      console.error('Move failed:', err);
      alert('Failed to move: ' + err);
    }
  }

  function cleanupDrag() {
    draggedNode = null;
    dropTargetPath = null;
  }

  // Context menu handlers
  function handleContextMenu(e: MouseEvent, node: FileNode) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, node };
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  // Clipboard operations
  function handleCopy(node: FileNode) {
    clipboardStore.copy(node);
  }

  function handleCut(node: FileNode) {
    clipboardStore.cut(node);
  }

  async function handlePaste(targetNode: FileNode) {
    const state = clipboardStore.getState();
    if (!state.node || !state.operation) return;

    const targetDir = targetNode.is_dir ? targetNode.path : '';

    try {
      if (state.operation === 'copy') {
        const command = state.node.is_dir ? 'copy_folder' : 'copy_file';
        await invoke(command, {
          source: state.node.path,
          targetDir: targetDir,
        });
      } else if (state.operation === 'cut') {
        const command = state.node.is_dir ? 'move_folder' : 'move_file';
        const newPath = await invoke<string>(command, {
          source: state.node.path,
          targetDir: targetDir,
        });

        // Update current file path if it was moved
        if (notesStore.currentFile === state.node.path) {
          notesStore.updateCurrentFilePath(newPath);
        }

        clipboardStore.clear();
      }

      await notesStore.loadFiles();
    } catch (err) {
      console.error('Paste failed:', err);
      alert('Failed to paste: ' + err);
    }
  }

  async function handleDelete(node: FileNode) {
    const confirmed = await confirm(
      `Delete "${node.name}"?`,
      { title: 'Delete', kind: 'warning' }
    );

    if (!confirmed) return;

    try {
      await invoke('note_delete', { path: node.path });

      // Close file if it was deleted
      if (notesStore.currentFile === node.path) {
        notesStore.closeFile();
      }

      await notesStore.loadFiles();
    } catch (err) {
      console.error('Delete failed:', err);
      alert('Failed to delete: ' + err);
    }
  }

  async function handleRename(node: FileNode) {
    const newName = prompt('Enter new name:', node.name);
    if (!newName || newName === node.name) return;

    try {
      // For files, ensure .md extension
      const finalName = !node.is_dir && !newName.endsWith('.md')
        ? `${newName}.md`
        : newName;

      const resultPath = await invoke<string>('rename_item', {
        oldPath: node.path,
        newName: finalName,
      });

      // Update current file path if it was renamed
      if (notesStore.currentFile === node.path) {
        notesStore.updateCurrentFilePath(resultPath);
      }

      await notesStore.loadFiles();
    } catch (err) {
      console.error('Rename failed:', err);
      alert('Failed to rename: ' + err);
    }
  }

  // Keyboard shortcuts
  function handleKeyDown(e: KeyboardEvent, node: FileNode) {
    // Ctrl+C - Copy
    if (e.ctrlKey && e.code === 'KeyC') {
      e.preventDefault();
      handleCopy(node);
      return;
    }

    // Ctrl+X - Cut
    if (e.ctrlKey && e.code === 'KeyX') {
      e.preventDefault();
      handleCut(node);
      return;
    }

    // Ctrl+V - Paste
    if (e.ctrlKey && e.code === 'KeyV') {
      e.preventDefault();
      if (clipboardStore.getState().node) {
        handlePaste(node);
      }
      return;
    }

    // Delete
    if (e.code === 'Delete') {
      e.preventDefault();
      handleDelete(node);
      return;
    }

    // F2 - Rename
    if (e.code === 'F2') {
      e.preventDefault();
      handleRename(node);
      return;
    }
  }
</script>

<div
  class="file-tree"
  class:drop-target-root={dropTargetPath === '' && level === 0}
  ondragover={handleDragOverRoot}
  ondrop={handleDropRoot}
  ondragleave={handleDragLeave}
>
  {#each nodes as node}
    <div class="file-node">
      <button
        class="file-button"
        class:active={isActive(node.path)}
        class:is-directory={node.is_dir}
        class:drop-target={dropTargetPath === node.path}
        style="padding-left: {12 + level * 12}px"
        draggable="true"
        ondragstart={(e) => handleDragStart(e, node)}
        ondragover={(e) => node.is_dir && handleDragOver(e, node)}
        ondrop={(e) => node.is_dir && handleDrop(e, node)}
        ondragleave={handleDragLeave}
        onclick={() => handleFileClick(node)}
        oncontextmenu={(e) => handleContextMenu(e, node)}
        onkeydown={(e) => handleKeyDown(e, node)}
        tabindex="0"
      >
        <span class="name">{node.name}</span>
      </button>

      {#if node.is_dir && expandedDirs.has(node.path) && node.children}
        <FileTree nodes={node.children} level={level + 1} />
      {/if}
    </div>
  {/each}
</div>

{#if contextMenu && level === 0}
  <ContextMenu
    x={contextMenu.x}
    y={contextMenu.y}
    node={contextMenu.node}
    onCopy={handleCopy}
    onCut={handleCut}
    onPaste={handlePaste}
    onDelete={handleDelete}
    onRename={handleRename}
    onClose={closeContextMenu}
    canPaste={clipboard.node !== null}
  />
{/if}

<style>
  .file-tree {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .file-node {
    display: flex;
    flex-direction: column;
  }

  .file-button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 10px;
    margin: 1px 8px;
    width: calc(100% - 16px);
    text-align: left;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid transparent;
    cursor: pointer;
    transition: all 0.18s cubic-bezier(0.25, 0.46, 0.45, 0.94);
    font-size: 13px;
    line-height: 1.5;
    font-weight: 400;
    border-radius: 8px;
  }

  .file-button:hover:not(.active) {
    background: rgba(255, 255, 255, 0.04);
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
    border: 1px solid rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
  }

  .file-button.active {
    /* Refined gradient - lighter on top, darker on bottom */
    background: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.09) 0%,
      rgba(255, 255, 255, 0.06) 100%
    );

    /* Apple-style backdrop blur */
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);

    /* Almost invisible border */
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 8px;

    /* Very soft, barely visible shadows */
    box-shadow:
      0 0.5px 2px rgba(0, 0, 0, 0.03),
      0 1px 3px rgba(0, 0, 0, 0.04),
      inset 0 0.5px 0 rgba(255, 255, 255, 0.12);

    /* Text styling - clean, not too bold */
    color: rgba(255, 255, 255, 0.95);
    font-weight: 500;

    /* Minimal elevation */
    transform: translateX(1px);
  }

  .file-button.is-directory {
    font-weight: 450;
    color: var(--text-primary);
  }

  .file-button.is-directory.active {
    font-weight: 500;
  }

  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Drag & Drop styles */
  .file-button.drop-target {
    background: rgba(122, 162, 247, 0.12);
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
    border: 1px solid rgba(122, 162, 247, 0.3);
    box-shadow: 0 2px 8px rgba(122, 162, 247, 0.2);
  }

  .file-tree.drop-target-root {
    background: var(--bg-hover);
    border: 2px dashed var(--accent);
    border-radius: 6px;
  }

  .file-button[draggable="true"] {
    cursor: grab;
  }

  .file-button[draggable="true"]:active {
    cursor: grabbing;
  }
</style>
