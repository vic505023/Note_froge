import { invoke } from '@tauri-apps/api/core';
import type { FileNode, AppConfig, ChatMessage, ChatHistoryEntry, NotebookInfo } from '../types';
import { withTimeout } from './async';

const DEFAULT_INVOKE_TIMEOUT_MS = 30_000;

async function invokeWithTimeout<T>(
  command: string,
  args?: Record<string, unknown>,
  timeoutMs: number = DEFAULT_INVOKE_TIMEOUT_MS
): Promise<T> {
  const invokePromise = args ? invoke<T>(command, args) : invoke<T>(command);
  return await withTimeout(invokePromise, timeoutMs, `Tauri invoke timed out: ${command}`);
}

// Vault commands
export async function vaultInit(path: string): Promise<string> {
  return await invokeWithTimeout<string>('vault_init', { path }, 60_000);
}

export async function vaultGetPath(): Promise<string> {
  return await invokeWithTimeout<string>('vault_get_path', undefined, 10_000);
}

// Notebook commands
export async function notebookList(): Promise<NotebookInfo[]> {
  return await invokeWithTimeout<NotebookInfo[]>('notebook_list', undefined, 60_000);
}

export async function notebookCreate(name: string): Promise<void> {
  await invokeWithTimeout('notebook_create', { name }, 30_000);
}

export async function notebookDelete(name: string): Promise<void> {
  await invokeWithTimeout('notebook_delete', { name }, 30_000);
}

export async function notebookRename(oldName: string, newName: string): Promise<void> {
  await invokeWithTimeout('notebook_rename', { oldName, newName }, 30_000);
}

// Note commands
export async function noteList(notebook: string): Promise<FileNode[]> {
  return await invokeWithTimeout<FileNode[]>('note_list', { notebook }, 60_000);
}

export async function noteRead(path: string): Promise<string> {
  return await invokeWithTimeout<string>('note_read', { path }, 30_000);
}

export async function noteWrite(path: string, content: string): Promise<void> {
  await invokeWithTimeout('note_write', { path, content }, 30_000);
}

export async function noteCreate(path: string): Promise<void> {
  await invokeWithTimeout('note_create', { path }, 30_000);
}

export async function noteDelete(path: string): Promise<void> {
  await invokeWithTimeout('note_delete', { path }, 30_000);
}

// Settings commands
export async function getSettings(): Promise<AppConfig> {
  return await invokeWithTimeout<AppConfig>('get_settings', undefined, 30_000);
}

export async function updateSettings(config: AppConfig): Promise<void> {
  await invokeWithTimeout('update_settings', { config }, 30_000);
}

// AI commands
export async function aiChat(
  messages: ChatMessage[],
  notebook: string,
  noteContext?: string,
  useSources: boolean = false,
  webSearch: boolean = false
): Promise<void> {
  await invokeWithTimeout('ai_chat', { messages, notebook, noteContext, useSources, webSearch }, 120_000);
}

export async function aiEditNote(
  instruction: string,
  currentContent: string,
  notebook: string,
  previousMessages: ChatMessage[]
): Promise<string> {
  return await invokeWithTimeout<string>(
    'ai_edit_note',
    {
      instruction,
      currentContent,
      notebook,
      previousMessages
    },
    120_000
  );
}

export async function aiTestConnection(): Promise<string> {
  return await invokeWithTimeout<string>('ai_test_connection', undefined, 30_000);
}

export async function saveChatMessage(
  notebook: string,
  notePath: string | null,
  role: string,
  content: string,
  mode: string
): Promise<void> {
  await invokeWithTimeout('save_chat_message', { notebook, notePath, role, content, mode }, 30_000);
}

export async function getChatHistory(
  notebook: string,
  notePath: string | null,
  mode: string,
  limit?: number
): Promise<ChatHistoryEntry[]> {
  return await invokeWithTimeout<ChatHistoryEntry[]>(
    'get_chat_history',
    { notebook, notePath, mode, limit },
    30_000
  );
}

export async function clearChatHistory(
  notebook: string,
  notePath: string | null,
  mode: string
): Promise<void> {
  await invokeWithTimeout('clear_chat_history', { notebook, notePath, mode }, 30_000);
}

export async function resolveWikiLink(target: string, notebook: string): Promise<string | null> {
  return await invokeWithTimeout<string | null>('resolve_wiki_link', { target, notebook }, 30_000);
}
