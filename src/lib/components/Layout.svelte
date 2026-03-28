<script lang="ts">
  import Header from './Header.svelte';
  import Sidebar from './Sidebar.svelte';
  import EditorPane from './EditorPane.svelte';
  import AIPanel from './AIPanel.svelte';
  import QuickOpen from './QuickOpen.svelte';
  import Settings from './Settings.svelte';
  import InputModal from './InputModal.svelte';
  import AddSourceModal from './AddSourceModal.svelte';
  import { uiStore } from '../stores/ui.svelte';
  import { notesStore } from '../stores/notes.svelte';
  import { notebooksStore } from '../stores/notebooks.svelte';
  import { aiStore } from '../stores/ai.svelte';
  import { settingsStore } from '../stores/settings.svelte';
  import { onMount } from 'svelte';

  let quickOpenVisible = $state(false);
  let settingsVisible = $state(false);
  let createNoteModalVisible = $state(false);

  let sidebarWidth = $state(250);
  let aiPanelWidth = $state(280);
  let isResizingSidebar = $state(false);
  let isResizingAI = $state(false);

  function handleSidebarResizeMouseDown(e: MouseEvent) {
    isResizingSidebar = true;
    e.preventDefault();
  }

  function handleAIResizeMouseDown(e: MouseEvent) {
    isResizingAI = true;
    e.preventDefault();
  }

  function handleResizeMouseMove(e: MouseEvent) {
    if (isResizingSidebar) {
      const newWidth = e.clientX;
      if (newWidth >= 250 && newWidth <= 500) {
        sidebarWidth = newWidth;
      }
    } else if (isResizingAI) {
      const newWidth = window.innerWidth - e.clientX;
      if (newWidth >= 240 && newWidth <= 600) {
        aiPanelWidth = newWidth;
      }
    }
  }

  function handleResizeMouseUp() {
    isResizingSidebar = false;
    isResizingAI = false;
  }

  async function handleCreateNote(name: string) {
    const currentNotebook = notebooksStore.currentNotebook;
    if (!currentNotebook) return;

    const fileName = name.endsWith('.md') ? name : `${name}.md`;
    const path = `${currentNotebook}/${fileName}`;

    try {
      await notesStore.createFile(path);
      await notesStore.loadFiles(currentNotebook);
    } catch (err) {
      console.error('Failed to create note:', err);
      alert('Failed to create note: ' + err);
    }
  }

  onMount(async () => {
    // Initialize settings
    await settingsStore.loadSettings();

    // Initialize AI store listeners
    await aiStore.init();

    // Listen for new-note event from TabBar
    function handleNewNote() {
      if (notebooksStore.currentNotebook) {
        createNoteModalVisible = true;
      }
    }
    window.addEventListener('new-note', handleNewNote);

    // Global keyboard shortcuts
    function handleKeyDown(e: KeyboardEvent) {
      // Skip if typing in input/textarea (except for specific shortcuts)
      const target = e.target as HTMLElement;
      const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA';

      // Ctrl+P: Quick Open (works everywhere)
      if (e.ctrlKey && e.code === 'KeyP') {
        e.preventDefault();
        quickOpenVisible = true;
        return;
      }

      // Ctrl+Shift+F: Search (open quick open)
      if (e.ctrlKey && e.shiftKey && e.code === 'KeyF') {
        e.preventDefault();
        quickOpenVisible = true;
        return;
      }

      // Ctrl+E: Cycle editor modes (works everywhere including editor)
      if (e.ctrlKey && e.code === 'KeyE' && !e.shiftKey && !e.altKey) {
        if (notesStore.currentFile) {
          e.preventDefault();
          e.stopPropagation();
          uiStore.cycleViewMode();
          console.log('View mode changed to:', uiStore.viewMode);
        }
        return;
      }

      // Ctrl+S: Force save (works everywhere including editor)
      if (e.ctrlKey && e.code === 'KeyS') {
        e.preventDefault();
        e.stopPropagation();
        notesStore.saveFile();
        return;
      }

      // Ctrl+N: New note (skip if typing in input)
      if (e.ctrlKey && e.code === 'KeyN' && !isInput) {
        e.preventDefault();
        if (notebooksStore.currentNotebook) {
          createNoteModalVisible = true;
        }
        return;
      }

      // Ctrl+B: Toggle sidebar (works everywhere)
      if (e.ctrlKey && e.code === 'KeyB') {
        e.preventDefault();
        uiStore.toggleSidebar();
        return;
      }

      // Ctrl+Shift+A: Toggle AI panel (works everywhere)
      if (e.ctrlKey && e.shiftKey && e.code === 'KeyA') {
        e.preventDefault();
        uiStore.toggleAIPanel();
        return;
      }

      // Ctrl+, (Comma): Open settings (works everywhere)
      if (e.ctrlKey && e.code === 'Comma') {
        e.preventDefault();
        settingsVisible = true;
        return;
      }

      // Ctrl+W: Close tab (works everywhere)
      if (e.ctrlKey && e.code === 'KeyW') {
        e.preventDefault();
        if (notesStore.activeTab) {
          notesStore.closeTab(notesStore.activeTab);
        }
        return;
      }

      // Ctrl+Tab: Next tab (works everywhere)
      if (e.ctrlKey && e.code === 'Tab' && !e.shiftKey) {
        e.preventDefault();
        notesStore.nextTab();
        return;
      }

      // Ctrl+Shift+Tab: Previous tab (works everywhere)
      if (e.ctrlKey && e.shiftKey && e.code === 'Tab') {
        e.preventDefault();
        notesStore.prevTab();
        return;
      }

      // Escape: Close modals
      if (e.code === 'Escape') {
        if (quickOpenVisible) {
          quickOpenVisible = false;
          return;
        }
        if (settingsVisible) {
          settingsVisible = false;
          return;
        }
        // If AI is streaming, stop it
        if (aiStore.isStreaming) {
          aiStore.stopStreaming();
          return;
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('mousemove', handleResizeMouseMove);
    window.addEventListener('mouseup', handleResizeMouseUp);

    return () => {
      window.removeEventListener('new-note', handleNewNote);
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('mousemove', handleResizeMouseMove);
      window.removeEventListener('mouseup', handleResizeMouseUp);
    };
  });
</script>

<div class="layout" style="--sidebar-width: {sidebarWidth}px; --ai-width: {aiPanelWidth}px">
  <Header {sidebarWidth} {aiPanelWidth} />

  <div class="main-content">
    {#if uiStore.sidebarOpen}
      <div class="sidebar">
        <Sidebar />
      </div>
      <div class="resizer" onmousedown={handleSidebarResizeMouseDown} class:resizing={isResizingSidebar}></div>
    {/if}

    <div class="editor-container">
      <EditorPane viewMode={uiStore.viewMode} />
    </div>

    {#if uiStore.aiPanelOpen}
      <div class="resizer" onmousedown={handleAIResizeMouseDown} class:resizing={isResizingAI}></div>
      <div class="ai-panel">
        <AIPanel />
      </div>
    {/if}
  </div>

  <!-- Quick Open Modal -->
  <QuickOpen bind:isOpen={quickOpenVisible} />

  <!-- Settings Modal -->
  <Settings isOpen={settingsVisible} onClose={() => settingsVisible = false} />

  <!-- Create Note Modal -->
  <InputModal
    bind:isOpen={createNoteModalVisible}
    title="Create Note"
    placeholder="Note name"
    onSubmit={handleCreateNote}
  />

  <!-- Add Source Modal -->
  {#if uiStore.addSourceModalOpen}
    <AddSourceModal onClose={() => uiStore.closeAddSourceModal()} />
  {/if}
</div>

<style>
  .layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background: var(--bg-base);
  }

  .main-content {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .sidebar {
    width: var(--sidebar-width);
    background: var(--glass-bg);
    backdrop-filter: blur(var(--glass-blur)) saturate(180%);
    -webkit-backdrop-filter: blur(var(--glass-blur)) saturate(180%);
    overflow: visible;
    flex-shrink: 0;
    border-right: 1px solid rgba(255, 255, 255, 0.08);
  }

  .editor-container {
    flex: 1;
    background: var(--bg-primary);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .resizer {
    width: 1px;
    background: var(--border-subtle);
    cursor: col-resize;
    flex-shrink: 0;
    position: relative;
    transition: background var(--transition-fast);
  }

  .resizer:hover,
  .resizer.resizing {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent-glow);
  }

  .ai-panel {
    width: var(--ai-width);
    background: var(--glass-bg);
    backdrop-filter: blur(var(--glass-blur)) saturate(180%);
    -webkit-backdrop-filter: blur(var(--glass-blur)) saturate(180%);
    overflow: hidden;
    flex-shrink: 0;
    border-left: 1px solid rgba(255, 255, 255, 0.08);
  }
</style>
