import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { DocumentInfo } from '../types';

export interface IndexingStats {
    total_notes: number;
    indexed_notes: number;
    total_documents: number;
    indexed_documents: number;
    total_chunks: number;
}

class DocumentsStore {
    documents = $state<DocumentInfo[]>([]);
    stats = $state<IndexingStats | null>(null);
    indexingInProgress = $state<Set<number>>(new Set());
    indexingErrors = $state<Map<number, string>>(new Map());
    isLoading = $state(false);
    error = $state<string | null>(null);

    constructor() {
        // Listen to indexing events
        listen<{ id: number; filename: string }>('document-indexing-start', (event) => {
            this.indexingInProgress.add(event.payload.id);
        });

        listen<{ id: number }>('document-indexed', (event) => {
            this.indexingInProgress.delete(event.payload.id);
            this.indexingErrors.delete(event.payload.id);

            // Update the document in the list to reflect indexed_at
            const docIndex = this.documents.findIndex(d => d.id === event.payload.id);
            if (docIndex !== -1) {
                this.documents[docIndex] = {
                    ...this.documents[docIndex],
                    indexed_at: Date.now() / 1000 // Current timestamp in seconds
                };
            }

            this.loadStats();
        });

        listen<{ id: number; error: string }>('document-indexing-error', (event) => {
            this.indexingInProgress.delete(event.payload.id);
            this.indexingErrors.set(event.payload.id, event.payload.error);
        });
    }

    async loadForNotebook(notebook: string) {
        this.isLoading = true;
        this.error = null;
        try {
            const docs = await invoke<DocumentInfo[]>('document_list_for_notebook', { notebook });
            this.documents = docs;
        } catch (err) {
            this.error = err instanceof Error ? err.message : String(err);
            console.error('Failed to load documents:', err);
            this.documents = [];
        } finally {
            this.isLoading = false;
        }
    }

    async loadAll() {
        this.isLoading = true;
        this.error = null;
        try {
            const docs = await invoke<DocumentInfo[]>('document_list_all');
            this.documents = docs;
        } catch (err) {
            this.error = err instanceof Error ? err.message : String(err);
            console.error('Failed to load all documents:', err);
            this.documents = [];
        } finally {
            this.isLoading = false;
        }
    }

    async upload(sourcePath: string, notebook: string): Promise<DocumentInfo> {
        // Call backend in background, document will appear when ready
        const doc = await invoke<DocumentInfo>('document_upload', { sourcePath, notebook });
        this.documents = [...this.documents, doc];
        return doc;
    }

    async remove(id: number) {
        await invoke('document_delete', { id });
        this.documents = this.documents.filter((d) => d.id !== id);
        this.indexingInProgress.delete(id);
        this.indexingErrors.delete(id);
    }

    async addToNotebook(documentId: number, notebook: string) {
        await invoke('document_add_to_notebook', { documentId, notebook });
    }

    async removeFromNotebook(documentId: number, notebook: string) {
        await invoke('document_remove_from_notebook', { documentId, notebook });
        this.documents = this.documents.filter((d) => d.id !== documentId);
    }

    async loadStats() {
        try {
            const stats = await invoke<IndexingStats>('get_indexing_stats');
            this.stats = stats;
        } catch (error) {
            console.error('Failed to load indexing stats:', error);
        }
    }

    getDocumentStatus(doc: DocumentInfo): 'indexed' | 'indexing' | 'error' | 'pending' {
        if (this.indexingErrors.has(doc.id)) return 'error';
        if (this.indexingInProgress.has(doc.id)) return 'indexing';
        if (doc.indexed_at) return 'indexed';
        return 'pending';
    }

    formatFileSize(bytes: number): string {
        if (bytes < 1024) return `${bytes}B`;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
        return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
    }

    clear() {
        this.documents = [];
        this.error = null;
    }
}

export const documentsStore = new DocumentsStore();
