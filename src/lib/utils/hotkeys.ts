import { notesStore } from '../stores/notes.svelte';
import { uiStore } from '../stores/ui.svelte';

export interface HotkeyHandler {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  handler: () => void;
  description: string;
}

export const hotkeys: HotkeyHandler[] = [
  {
    key: 'p',
    ctrl: true,
    description: 'Quick Open',
    handler: () => {
      // Toggle quick open modal
      const event = new CustomEvent('toggle-quick-open');
      window.dispatchEvent(event);
    }
  },
  {
    key: 'f',
    ctrl: true,
    shift: true,
    description: 'FTS Search',
    handler: () => {
      const event = new CustomEvent('focus-search');
      window.dispatchEvent(event);
    }
  },
  {
    key: 'e',
    ctrl: true,
    description: 'Cycle view mode',
    handler: () => {
      const event = new CustomEvent('cycle-view-mode');
      window.dispatchEvent(event);
    }
  },
  {
    key: 's',
    ctrl: true,
    description: 'Save file',
    handler: () => {
      if (notesStore.currentFile) {
        notesStore.saveFile().catch(err => {
          console.error('Failed to save:', err);
        });
      }
    }
  },
  {
    key: 'n',
    ctrl: true,
    description: 'New note',
    handler: () => {
      const event = new CustomEvent('new-note');
      window.dispatchEvent(event);
    }
  },
  {
    key: 'b',
    ctrl: true,
    description: 'Toggle sidebar',
    handler: () => {
      uiStore.toggleSidebar();
    }
  },
  {
    key: 'a',
    ctrl: true,
    shift: true,
    description: 'Toggle AI panel',
    handler: () => {
      uiStore.toggleAIPanel();
    }
  },
  {
    key: ',',
    ctrl: true,
    description: 'Settings',
    handler: () => {
      const event = new CustomEvent('open-settings');
      window.dispatchEvent(event);
    }
  },
  {
    key: 'w',
    ctrl: true,
    description: 'Close tab',
    handler: () => {
      if (notesStore.activeTab) {
        notesStore.closeTab(notesStore.activeTab);
      }
    }
  },
  {
    key: 'Tab',
    ctrl: true,
    description: 'Next tab',
    handler: () => {
      notesStore.nextTab();
    }
  },
  {
    key: 'Tab',
    ctrl: true,
    shift: true,
    description: 'Previous tab',
    handler: () => {
      notesStore.prevTab();
    }
  },
  {
    key: 'Enter',
    ctrl: true,
    description: 'Send message (in chat)',
    handler: () => {
      const event = new CustomEvent('send-chat-message');
      window.dispatchEvent(event);
    }
  },
  {
    key: 'Escape',
    description: 'Close modal / Stop AI',
    handler: () => {
      const event = new CustomEvent('escape-pressed');
      window.dispatchEvent(event);
    }
  }
];

export function handleKeyDown(event: KeyboardEvent): boolean {
  for (const hotkey of hotkeys) {
    const keyMatch = event.key === hotkey.key;
    const ctrlMatch = hotkey.ctrl ? event.ctrlKey || event.metaKey : !event.ctrlKey && !event.metaKey;
    const shiftMatch = hotkey.shift ? event.shiftKey : !event.shiftKey;
    const altMatch = hotkey.alt ? event.altKey : !event.altKey;

    if (keyMatch && ctrlMatch && shiftMatch && altMatch) {
      // Don't handle hotkeys when typing in input/textarea
      const target = event.target as HTMLElement;
      if (
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable
      ) {
        // Allow Ctrl+S in editor
        if (event.key === 's' && event.ctrlKey) {
          event.preventDefault();
          hotkey.handler();
          return true;
        }
        // Allow Escape
        if (event.key === 'Escape') {
          event.preventDefault();
          hotkey.handler();
          return true;
        }
        return false;
      }

      event.preventDefault();
      hotkey.handler();
      return true;
    }
  }

  return false;
}
