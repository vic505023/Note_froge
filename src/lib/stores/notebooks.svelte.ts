import type { NotebookInfo } from '../types';
import { notebookCreate, notebookDelete, notebookList, notebookRename } from '../utils/tauri';

class NotebooksStore {
    notebooks = $state<NotebookInfo[]>([]);
    currentNotebook = $state<string | null>(null);
    isLoading = $state(false);
    error = $state<string | null>(null);

    async loadNotebooks() {
        this.isLoading = true;
        this.error = null;
        try {
            const notebooks = await notebookList();
            this.notebooks = notebooks;
        } catch (err) {
            this.error = err instanceof Error ? err.message : String(err);
            console.error('Failed to load notebooks:', err);
        } finally {
            this.isLoading = false;
        }
    }

    async selectNotebook(name: string) {
        this.currentNotebook = name;
        // Trigger notes loading for this notebook
        const { notesStore } = await import('./notes.svelte');
        await notesStore.loadFiles(name);
    }

    async createNotebook(name: string) {
        this.error = null;
        try {
            await notebookCreate(name);
            await this.loadNotebooks();
            await this.selectNotebook(name);
        } catch (err) {
            this.error = err instanceof Error ? err.message : String(err);
            console.error('Failed to create notebook:', err);
            throw err;
        }
    }

    async deleteNotebook(name: string) {
        this.error = null;
        try {
            await notebookDelete(name);
            if (this.currentNotebook === name) {
                this.currentNotebook = null;
            }
            await this.loadNotebooks();
        } catch (err) {
            this.error = err instanceof Error ? err.message : String(err);
            console.error('Failed to delete notebook:', err);
            throw err;
        }
    }

    async renameNotebook(oldName: string, newName: string) {
        this.error = null;
        try {
            await notebookRename(oldName, newName);
            if (this.currentNotebook === oldName) {
                this.currentNotebook = newName;
            }
            await this.loadNotebooks();
        } catch (err) {
            this.error = err instanceof Error ? err.message : String(err);
            console.error('Failed to rename notebook:', err);
            throw err;
        }
    }

    deselectNotebook() {
        this.currentNotebook = null;
    }
}

export const notebooksStore = new NotebooksStore();
