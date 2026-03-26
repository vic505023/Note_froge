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
        // Listen to parsing event (updates title after Vision OCR)
        listen<{ id: number; title: string; page_count: number | null }>('document-parsed', (event) => {
            console.log('📘 EVENT: document-parsed', event.payload.id);
            const docIndex = this.documents.findIndex(d => d.id === event.payload.id);
            if (docIndex !== -1) {
                console.log('📘 Updating document at index', docIndex);
                this.documents[docIndex] = {
                    ...this.documents[docIndex],
                    title: event.payload.title,
                    page_count: event.payload.page_count
                };
                console.log('📘 Documents array after update:', this.documents.map(d => ({ id: d.id, title: d.title })));
            }
        });

        // Listen to indexing events
        listen<{ id: number; filename: string }>('document-indexing-start', (event) => {
            this.indexingInProgress.add(event.payload.id);
        });

        listen<{ id: number }>('document-indexed', (event) => {
            console.log('📗 EVENT: document-indexed', event.payload.id);
            this.indexingInProgress.delete(event.payload.id);
            this.indexingErrors.delete(event.payload.id);

            // Update the document in the list to reflect indexed_at
            const docIndex = this.documents.findIndex(d => d.id === event.payload.id);
            if (docIndex !== -1) {
                console.log('📗 Updating document at index', docIndex);
                this.documents[docIndex] = {
                    ...this.documents[docIndex],
                    indexed_at: Date.now() / 1000 // Current timestamp in seconds
                };
                console.log('📗 Documents array after update:', this.documents.map(d => ({ id: d.id, indexed: !!d.indexed_at })));
            }

            this.loadStats();
        });

        listen<{ id: number; error: string }>('document-indexing-error', (event) => {
            this.indexingInProgress.delete(event.payload.id);
            this.indexingErrors.set(event.payload.id, event.payload.error);
        });

        listen<{ id: number; error: string }>('document-parse-error', (event) => {
            this.indexingErrors.set(event.payload.id, `Parse error: ${event.payload.error}`);
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
        // Document appears immediately with temporary title (filename)
        // Title will be updated after parsing via 'document-parsed' event
        // Indexing happens in background
        const doc = await invoke<DocumentInfo>('document_upload', { sourcePath, notebook });
        this.documents = [...this.documents, doc];
        this.indexingInProgress.add(doc.id); // Mark as indexing
        return doc;
    }

    async remove(id: number) {
        console.trace('🔴 documentsStore.remove() CALLED for id:', id);
        console.log('🔴 Documents BEFORE filter:', this.documents.map(d => d.id));
        await invoke('document_delete', { id });
        this.documents = this.documents.filter((d) => d.id !== id);
        console.log('🔴 Documents AFTER filter:', this.documents.map(d => d.id));
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
