export interface FileNode {
  name: string;
  path: string;
  is_dir: boolean;
  children?: FileNode[];
}

export interface NotebookInfo {
  name: string;
  note_count: number;
  document_count: number;
  created_at: number;
}

export interface DocumentInfo {
  id: number;
  filename: string;
  filepath: string;
  file_type: 'pdf' | 'docx' | 'pptx' | 'txt';
  title: string;
  page_count: number | null;
  size_bytes: number;
  indexed_at: number | null;
  created_at: number;
}

export interface AppConfig {
  vault: VaultConfig;
  ai: AIConfig;
  agents: AIAgent[];
  active_agent_id: string | null;
  vision: VisionConfig;
  editor: EditorConfig;
  ui: UIConfig;
}

export interface VaultConfig {
  path: string | null;
}

export interface AIConfig {
  base_url: string;
  api_key: string;
  model: string;
  embedding_model: string;
}

export interface AIAgent {
  id: string;
  name: string;
  base_url: string;
  api_key: string;
  models: string[];
  embedding_model: string;
  embedding_base_url?: string;
  embedding_api_key?: string;
}

export interface VisionConfig {
  enabled: boolean;
  base_url: string;
  api_key: string;
  model: string;
}

export interface EditorConfig {
  font_size: number;
  font_family: string;
  theme: string;
  autosave_ms: number;
}

export interface UIConfig {
  sidebar_width: number;
  ai_panel_width: number;
  ai_panel_open: boolean;
  sidebar_open: boolean;
}

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

export interface SearchResult {
  path: string;
  title: string;
  snippet: string;
}

export interface ChatHistoryEntry {
  id: number;
  note_path: string | null;
  role: string;
  content: string;
  mode: string;
  created_at: number;
}

export interface BacklinkResult {
  source_path: string;
  source_title: string;
  link_type: string;
}
