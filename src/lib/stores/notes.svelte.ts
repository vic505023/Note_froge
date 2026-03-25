import { invoke } from '@tauri-apps/api/core';
import type { FileNode } from '../types';

class NotesStore {
  files = $state<FileNode[]>([]);
  currentFile = $state<string | null>(null);
  currentContent = $state('');
  isLoading = $state(false);
  error = $state<string | null>(null);
  aiEditPending = $state(false);

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
    console.log('Opening file:', path);
    this.isLoading = true;
    this.error = null;
    try {
      const content = await invoke<string>('note_read', { path });
      console.log('File content loaded:', content.substring(0, 100));
      this.currentFile = path;
      this.currentContent = content;
      console.log('File opened successfully');
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
      console.error('Failed to open file:', err);
    } finally {
      this.isLoading = false;
    }
  }

  async saveFile() {
    if (!this.currentFile) return;

    try {
      await invoke('note_write', {
        path: this.currentFile,
        content: this.currentContent
      });
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
      console.error('Failed to save file:', err);
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

  updateContent(content: string) {
    this.currentContent = content;
  }

  applyAIEdit(content: string) {
    this.currentContent = content;
    this.aiEditPending = true;
  }

  clearAIEditFlag() {
    this.aiEditPending = false;
  }

  updateCurrentFilePath(newPath: string) {
    this.currentFile = newPath;
  }

  closeFile() {
    this.currentFile = null;
    this.currentContent = '';
  }
}

export const notesStore = new NotesStore();
