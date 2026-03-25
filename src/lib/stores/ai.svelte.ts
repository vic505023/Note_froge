import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { ChatMessage } from '../types';

interface Source {
  filename: string;
  relevance: number;
  page?: number;  // Optional page/slide number
}

class AIStore {
  messages = $state<ChatMessage[]>([]);
  isStreaming = $state(false);
  currentStreamContent = $state('');
  editMode = $state(false);
  error = $state<string | null>(null);
  currentNotePath = $state<string | null>(null);
  isEditingNote = $state(false);
  sources = $state<Source[]>([]);

  private unlistenChunk: UnlistenFn | null = null;
  private unlistenDone: UnlistenFn | null = null;
  private unlistenError: UnlistenFn | null = null;
  private unlistenSources: UnlistenFn | null = null;
  private isInitialized = false;

  async init() {
    if (this.isInitialized) {
      console.log('AI store already initialized, skipping');
      return;
    }
    this.isInitialized = true;
    console.log('Initializing AI store listeners');

    this.unlistenChunk = await listen<{ content: string }>('ai-chunk', (event) => {
      console.log('AI chunk received:', event.payload.content);
      this.currentStreamContent += event.payload.content;
      console.log('Total content length:', this.currentStreamContent.length);
    });

    this.unlistenDone = await listen('ai-done', async () => {
      const currentContent = this.currentStreamContent;
      const notePath = this.currentNotePath;

      console.log('AI done, saving assistant message. Content length:', currentContent.length, 'notePath:', notePath);

      // Save to database
      if (notePath && currentContent) {
        try {
          await invoke('save_chat_message', {
            notebook: notePath.split('/')[0],
            notePath,
            role: 'assistant',
            content: currentContent,
            mode: 'chat'
          });
          console.log('Assistant message saved to DB');
        } catch (err) {
          console.error('Failed to save assistant message:', err);
        }
      }

      // Update state
      this.messages = [
        ...this.messages,
        { role: 'assistant', content: currentContent }
      ];
      this.isStreaming = false;
      this.currentStreamContent = '';
    });

    this.unlistenError = await listen<{ error: string }>('ai-error', (event) => {
      this.isStreaming = false;
      this.currentStreamContent = '';
      this.error = event.payload.error;
    });

    this.unlistenSources = await listen<{ sources: Source[] }>('ai-sources', (event) => {
      console.log('AI sources received:', event.payload.sources);
      this.sources = event.payload.sources;
    });
  }

  async loadHistory(notePath: string | null) {
    try {
      console.log('Loading chat history for:', notePath);
      const history = await invoke<any[]>('get_chat_history', {
        notebook: notePath?.split('/')[0] || null,
        notePath,
        mode: 'chat',
        limit: 50
      });
      console.log('Loaded history entries:', history.length);
      this.messages = history.map(h => ({
        role: h.role as 'user' | 'assistant' | 'system',
        content: h.content
      }));
    } catch (err) {
      console.error('Failed to load chat history:', err);
    }
  }

  async sendMessage(
    content: string,
    noteContext: string | null,
    notePath: string | null,
    model?: string,
    useSources: boolean = true,
    webSearch: boolean = false
  ) {
    const userMessage: ChatMessage = { role: 'user', content };

    this.messages = [...this.messages, userMessage];
    this.isStreaming = true;
    this.currentStreamContent = '';
    this.error = null;
    this.currentNotePath = notePath;

    try {
      // Save user message to history
      console.log('Saving user message to history, notePath:', notePath);
      if (notePath) {
        await invoke('save_chat_message', {
          notebook: notePath.split('/')[0],
          notePath,
          role: 'user',
          content,
          mode: 'chat'
        });
        console.log('User message saved');
      }

      // Get all messages for API call (excluding system messages)
      const apiMessages = this.messages.filter(m => m.role !== 'system');

      // Send to API
      await invoke('ai_chat', {
        messages: apiMessages,
        notebook: notePath?.split('/')[0] || '',
        noteContext: noteContext || null,
        useSources,
        webSearch,
        model: model || null
      });
      // Assistant message will be saved by the 'ai-done' listener

    } catch (err) {
      this.isStreaming = false;
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  async editNote(instruction: string, currentContent: string, notePath: string | null, model?: string): Promise<string> {
    const userMessage: ChatMessage = { role: 'user', content: instruction };

    // Get current conversation history
    const previousMessages = this.messages.filter(m => m.role !== 'system');

    // Add user instruction to history
    this.messages = [...this.messages, userMessage];
    this.error = null;
    this.currentNotePath = notePath;
    this.isEditingNote = true;

    try {
      // Save user message to history
      if (notePath) {
        await invoke('save_chat_message', {
          notebook: notePath.split('/')[0],
          notePath,
          role: 'user',
          content: instruction,
          mode: 'chat'
        });
      }

      // Call edit API
        const newContent = await invoke<string>('ai_edit_note', {
            instruction,
            currentContent,
            notebook: notePath?.split('/')[0] || '',
            previousMessages,
            model: model || null
        });

      this.isEditingNote = false;
      return newContent;
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
      this.isEditingNote = false;
      throw err;
    }
  }

  async clearHistory(notePath: string | null) {
    try {
      await invoke('clear_chat_history', {
        notebook: notePath?.split('/')[0] || null,
        notePath,
        mode: 'chat'
      });
      this.messages = [];
    } catch (err) {
      console.error('Failed to clear chat history:', err);
    }
  }

  async confirmEdit(notePath: string | null) {
    const assistantMessage: ChatMessage = {
      role: 'assistant',
      content: `✏️ **Edited note**\n\nI've updated the note based on your request.`
    };

    this.messages = [...this.messages, assistantMessage];

    // Save to database
    if (notePath) {
      try {
        await invoke('save_chat_message', {
          notebook: notePath.split('/')[0],
          notePath,
          role: 'assistant',
          content: assistantMessage.content,
          mode: 'chat'
        });
      } catch (err) {
        console.error('Failed to save assistant message:', err);
      }
    }
  }

  async rejectEdit(notePath: string | null) {
    const assistantMessage: ChatMessage = {
      role: 'assistant',
      content: `Changes were rejected.`
    };

    this.messages = [...this.messages, assistantMessage];

    // Save to database
    if (notePath) {
      try {
        await invoke('save_chat_message', {
          notebook: notePath.split('/')[0],
          notePath,
          role: 'assistant',
          content: assistantMessage.content,
          mode: 'chat'
        });
      } catch (err) {
        console.error('Failed to save assistant message:', err);
      }
    }
  }

  toggleEditMode() {
    this.editMode = !this.editMode;
  }

  clearError() {
    this.error = null;
  }

  stopStreaming() {
    // TODO: Implement stream cancellation
    this.isStreaming = false;
    this.currentStreamContent = '';
  }
}

export const aiStore = new AIStore();
