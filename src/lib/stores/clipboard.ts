import { writable } from 'svelte/store';
import type { FileNode } from '../types';

type ClipboardOperation = 'copy' | 'cut';

interface ClipboardState {
  node: FileNode | null;
  operation: ClipboardOperation | null;
}

const initialState: ClipboardState = {
  node: null,
  operation: null,
};

const clipboardState = writable<ClipboardState>(initialState);

export const clipboardStore = {
  subscribe: clipboardState.subscribe,

  copy(node: FileNode) {
    clipboardState.set({ node, operation: 'copy' });
  },

  cut(node: FileNode) {
    clipboardState.set({ node, operation: 'cut' });
  },

  clear() {
    clipboardState.set(initialState);
  },

  getState(): ClipboardState {
    let state: ClipboardState = initialState;
    clipboardState.subscribe(s => state = s)();
    return state;
  }
};
