use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub config: Arc<RwLock<AppConfig>>,
    pub vault_path: Arc<RwLock<Option<PathBuf>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub vault: VaultConfig,
    #[serde(default)]
    pub ai: AIConfig, // Kept for backward compatibility
    #[serde(default)]
    pub agents: Vec<AIAgent>,
    #[serde(default)]
    pub active_agent_id: Option<String>,
    #[serde(default)]
    pub vision: VisionConfig,
    #[serde(default)]
    pub whisper: WhisperConfig,
    pub editor: EditorConfig,
    pub ui: UIConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            api_key: String::new(),
            model: default_model(),
            embedding_model: default_embedding_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAgent {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    #[serde(default = "default_models")]
    pub models: Vec<String>,
    pub embedding_model: String,
    #[serde(default)]
    pub embedding_base_url: Option<String>,
    #[serde(default)]
    pub embedding_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub base_url: String, // Empty = use main AI base_url
    #[serde(default)]
    pub api_key: String, // Empty = use main AI api_key
    #[serde(default = "default_vision_model")]
    pub model: String,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: String::new(),
            api_key: String::new(),
            model: default_vision_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub base_url: String, // Empty = use main AI base_url
    #[serde(default)]
    pub api_key: String, // Empty = use main AI api_key
    #[serde(default = "default_whisper_model")]
    pub model: String,
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: String::new(),
            api_key: String::new(),
            model: default_whisper_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    #[serde(default = "default_font_size")]
    pub font_size: u32,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_autosave_ms")]
    pub autosave_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u32,
    #[serde(default = "default_ai_panel_width")]
    pub ai_panel_width: u32,
    #[serde(default = "default_true")]
    pub ai_panel_open: bool,
    #[serde(default = "default_true")]
    pub sidebar_open: bool,
    #[serde(default)]
    pub selected_model: Option<String>,
}

fn default_base_url() -> String {
    "https://routerapi.ru/api/v1".to_string()
}

fn default_model() -> String {
    "gpt-4o".to_string()
}

fn default_models() -> Vec<String> {
    vec![]
}

fn default_embedding_model() -> String {
    String::new()
}

fn default_vision_model() -> String {
    "openai/gpt-4o-mini".to_string()
}

fn default_whisper_model() -> String {
    "whisper-1".to_string()
}

fn default_font_size() -> u32 {
    14
}

fn default_font_family() -> String {
    "JetBrains Mono".to_string()
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_autosave_ms() -> u64 {
    1000
}

fn default_sidebar_width() -> u32 {
    250
}

fn default_ai_panel_width() -> u32 {
    350
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            vault: VaultConfig { path: None },
            ai: AIConfig {
                base_url: default_base_url(),
                api_key: String::new(),
                model: default_model(),
                embedding_model: default_embedding_model(),
            },
            agents: vec![],
            active_agent_id: None,
            vision: VisionConfig::default(),
            whisper: WhisperConfig::default(),
            editor: EditorConfig {
                font_size: default_font_size(),
                font_family: default_font_family(),
                theme: default_theme(),
                autosave_ms: default_autosave_ms(),
            },
            ui: UIConfig {
                sidebar_width: default_sidebar_width(),
                ai_panel_width: default_ai_panel_width(),
                ai_panel_open: true,
                sidebar_open: true,
                selected_model: None,
            },
        }
    }
}

// Alias for compatibility with CLAUDE.md instructions
pub type AiConfig = AIConfig;

impl AIAgent {
    /// Get base URL for embeddings (uses separate URL if set, otherwise main URL)
    pub fn get_embedding_base_url(&self) -> String {
        self.embedding_base_url.as_ref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.base_url)
            .clone()
    }

    /// Get API key for embeddings (uses separate key if set, otherwise main key)
    pub fn get_embedding_api_key(&self) -> String {
        self.embedding_api_key.as_ref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.api_key)
            .clone()
    }
}

impl AppConfig {
    pub fn get_active_agent(&self) -> Option<&AIAgent> {
        if let Some(agent_id) = &self.active_agent_id {
            self.agents.iter().find(|a| &a.id == agent_id)
        } else {
            self.agents.first()
        }
    }

    pub fn get_active_agent_or_legacy(&self) -> AIAgent {
        if let Some(agent) = self.get_active_agent() {
            agent.clone()
        } else {
            // Fallback to legacy ai config
            AIAgent {
                id: "legacy".to_string(),
                name: "Legacy".to_string(),
                base_url: self.ai.base_url.clone(),
                api_key: self.ai.api_key.clone(),
                models: vec![self.ai.model.clone()],
                embedding_model: self.ai.embedding_model.clone(),
                embedding_base_url: None,
                embedding_api_key: None,
            }
        }
    }

    pub fn get_active_model(&self) -> Option<String> {
        if let Some(agent) = self.get_active_agent() {
            agent.models.first().cloned()
        } else if !self.ai.model.is_empty() {
            Some(self.ai.model.clone())
        } else {
            None
        }
    }

    /// Get vision base URL (uses separate URL if set, otherwise falls back to active agent)
    pub fn get_vision_base_url(&self) -> String {
        if !self.vision.base_url.is_empty() {
            self.vision.base_url.clone()
        } else {
            let agent = self.get_active_agent_or_legacy();
            agent.base_url
        }
    }

    /// Get vision API key (uses separate key if set, otherwise falls back to active agent)
    pub fn get_vision_api_key(&self) -> String {
        if !self.vision.api_key.is_empty() {
            self.vision.api_key.clone()
        } else {
            let agent = self.get_active_agent_or_legacy();
            agent.api_key
        }
    }

    /// Get whisper base URL (uses separate URL if set, otherwise falls back to active agent)
    pub fn get_whisper_base_url(&self) -> String {
        if !self.whisper.base_url.is_empty() {
            self.whisper.base_url.clone()
        } else {
            let agent = self.get_active_agent_or_legacy();
            agent.base_url
        }
    }

    /// Get whisper API key (uses separate key if set, otherwise falls back to active agent)
    pub fn get_whisper_api_key(&self) -> String {
        if !self.whisper.api_key.is_empty() {
            self.whisper.api_key.clone()
        } else {
            let agent = self.get_active_agent_or_legacy();
            agent.api_key
        }
    }
}
