import { invoke } from '@tauri-apps/api/core';
import type { FileNode } from '../types';

interface OpenTab {
  path: string;         // "Физика/note.md" or empty for empty tabs
  name: string;         // "note.md" or "New Tab"
  content: string;      // File content
  modified: boolean;    // Has unsaved changes
  savedContent: string; // Last saved content (for detecting changes)
  isEmpty?: boolean;    // True for empty tabs
}

class NotesStore {
  files = $state<FileNode[]>([]);
  tabs = $state<OpenTab[]>([]);
  activeTab = $state<string | null>(null);  // path of active tab
  isLoading = $state(false);
  error = $state<string | null>(null);
  aiEditPending = $state(false);

  // Getter for current file path (for backwards compatibility)
  get currentFile(): string | null {
    return this.activeTab;
  }

  // Getter for current content
  get currentContent(): string {
    const tab = this.getTab(this.activeTab);
    return tab?.content ?? '';
  }

  async loadFiles(notebook: string) {
    this.isLoading = true;
    this.error = null;
    try {
      const files = await invoke<FileNode[]>('note_list', { notebook });
      this.files = files;
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
      console.error('Failed to load files:', err);
    } finally {
      this.isLoading = false;
    }
  }

  async openFile(path: string) {
    // Check if tab already exists
    const existingTab = this.getTab(path);
    if (existingTab) {
      this.activeTab = path;
      return;
    }

    console.log('Opening file:', path);
    this.isLoading = true;
    this.error = null;
    try {
      const content = await invoke<string>('note_read', { path });
      console.log('File content loaded:', content.substring(0, 100));

      // Extract filename from path
      const name = path.split('/').pop() || path;

      // Check if current active tab is empty - replace it
      const currentTab = this.getTab(this.activeTab);
      if (currentTab?.isEmpty) {
        // Replace empty tab with file content
        currentTab.path = path;
        currentTab.name = name;
        currentTab.content = content;
        currentTab.savedContent = content;
        currentTab.modified = false;
        currentTab.isEmpty = false;
        this.activeTab = path;
        // Trigger reactivity
        this.tabs = [...this.tabs];
      } else {
        // Add new tab
        const newTab: OpenTab = {
          path,
          name,
          content,
          modified: false,
          savedContent: content
        };

        this.tabs = [...this.tabs, newTab];
        this.activeTab = path;
      }

      console.log('File opened successfully');
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
      console.error('Failed to open file:', err);
    } finally {
      this.isLoading = false;
    }
  }

  async saveFile(path?: string) {
    const targetPath = path || this.activeTab;
    if (!targetPath) return;

    const tab = this.getTab(targetPath);
    if (!tab) return;

    try {
      await invoke('note_write', {
        path: targetPath,
        content: tab.content
      });

      // Update saved content and clear modified flag
      tab.savedContent = tab.content;
      tab.modified = false;
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
      console.error('Failed to save file:', err);
      throw err;
    }
  }

  async createFile(path: string) {
    this.error = null;
    try {
      await invoke('note_create', { path });
      // Extract notebook from path for reload
      const notebook = path.split('/')[0];
      await this.loadFiles(notebook);
      await this.openFile(path);
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
      console.error('Failed to create file:', err);
      throw err;
    }
  }

  updateContent(content: string, path?: string) {
    const targetPath = path || this.activeTab;
    if (!targetPath) return;

    const tab = this.getTab(targetPath);
    if (!tab) return;

    tab.content = content;
    tab.modified = content !== tab.savedContent;
  }

  applyAIEdit(content: string, path?: string) {
    const targetPath = path || this.activeTab;
    if (!targetPath) return;

    const tab = this.getTab(targetPath);
    if (!tab) return;

    tab.content = content;
    tab.modified = content !== tab.savedContent;
    this.aiEditPending = true;
  }

  clearAIEditFlag() {
    this.aiEditPending = false;
  }

  updateCurrentFilePath(newPath: string) {
    // Update tab path when file is renamed
    const tab = this.getTab(this.activeTab);
    if (!tab) return;

    // Remove old tab
    this.tabs = this.tabs.filter(t => t.path !== this.activeTab);

    // Add new tab with updated path
    tab.path = newPath;
    tab.name = newPath.split('/').pop() || newPath;
    this.tabs.push(tab);
    this.activeTab = newPath;
  }

  closeTab(path: string) {
    const tab = this.getTab(path);
    if (!tab) return;

    // Check for unsaved changes
    if (tab.modified) {
      const confirmed = confirm(`"${tab.name}" has unsaved changes. Close anyway?`);
      if (!confirmed) return;
    }

    // Remove tab
    this.tabs = this.tabs.filter(t => t.path !== path);

    // If closing active tab, switch to another
    if (this.activeTab === path) {
      if (this.tabs.length > 0) {
        // Try to activate the next tab, or previous if this was the last
        const index = this.tabs.findIndex(t => t.path === path);
        const nextIndex = index < this.tabs.length ? index : this.tabs.length - 1;
        this.activeTab = this.tabs[nextIndex]?.path ?? null;
      } else {
        this.activeTab = null;
      }
    }
  }

  closeFile() {
    // For backwards compatibility - close active tab
    if (this.activeTab) {
      this.closeTab(this.activeTab);
    }
  }

  switchTab(path: string) {
    if (this.getTab(path)) {
      this.activeTab = path;
    }
  }

  nextTab() {
    if (this.tabs.length === 0) return;
    const currentIndex = this.tabs.findIndex(t => t.path === this.activeTab);
    const nextIndex = (currentIndex + 1) % this.tabs.length;
    this.activeTab = this.tabs[nextIndex].path;
  }

  prevTab() {
    if (this.tabs.length === 0) return;
    const currentIndex = this.tabs.findIndex(t => t.path === this.activeTab);
    const prevIndex = currentIndex <= 0 ? this.tabs.length - 1 : currentIndex - 1;
    this.activeTab = this.tabs[prevIndex].path;
  }

  private getTab(path: string | null): OpenTab | undefined {
    if (!path) return undefined;
    return this.tabs.find(t => t.path === path);
  }

  openEmptyTab() {
    // Create unique path for empty tab
    const emptyPath = `__empty__${Date.now()}`;

    const newTab: OpenTab = {
      path: emptyPath,
      name: 'New Tab',
      content: '',
      modified: false,
      savedContent: '',
      isEmpty: true
    };

    // Use spread to trigger reactivity
    this.tabs = [...this.tabs, newTab];
    this.activeTab = emptyPath;
  }

  // Mark file content as saved (after external save)
  markAsSaved(path?: string) {
    const targetPath = path || this.activeTab;
    if (!targetPath) return;

    const tab = this.getTab(targetPath);
    if (!tab) return;

    tab.savedContent = tab.content;
    tab.modified = false;
  }
}

export const notesStore = new NotesStore();
