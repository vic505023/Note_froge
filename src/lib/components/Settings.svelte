<script lang="ts">
  import { onMount } from 'svelte';
  import { getSettings, aiTestConnection } from '../utils/tauri';
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';
  import ConfirmModal from './ConfirmModal.svelte';
  import { settingsStore } from '../stores/settings.svelte';
  import type { AppConfig } from '../types';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();

  let config = $state<AppConfig | null>(null);
  let originalConfig = $state<AppConfig | null>(null);
  let showApiKey = $state(false);
  let showVisionApiKey = $state(false);
  let testStatus = $state<'idle' | 'testing' | 'success' | 'error'>('idle');
  let testError = $state('');
  let isReindexing = $state(false);
  let showReindexModal = $state(false);

  let expandedSections = $state({
    vault: true,
    ai: true,
    vision: true,
    editor: true
  });

  let editingAgent = $state<{ index: number; agent: typeof config.agents[0] } | null>(null);
  let newAgentName = $state('');
  let newModelInput = $state('');

  function toggleSection(section: 'vault' | 'ai' | 'vision' | 'editor') {
    expandedSections[section] = !expandedSections[section];
  }

  function incrementNumber(field: 'font_size' | 'autosave_ms', step: number) {
    if (!config) return;

    if (field === 'font_size') {
      const newValue = config.editor.font_size + step;
      if (newValue >= 10 && newValue <= 24) {
        config.editor.font_size = newValue;
      }
    } else if (field === 'autosave_ms') {
      const newValue = config.editor.autosave_ms + step;
      if (newValue >= 500 && newValue <= 10000) {
        config.editor.autosave_ms = newValue;
      }
    }
  }

  onMount(async () => {
    await loadSettings();
  });

  async function loadSettings() {
    try {
      const settings = await getSettings();
      config = JSON.parse(JSON.stringify(settings)); // Deep copy
      originalConfig = JSON.parse(JSON.stringify(settings)); // Deep copy
    } catch (err) {
      console.error('Failed to load settings:', err);
    }
  }

  async function handleSave() {
    if (!config) return;

    try {
      await settingsStore.saveSettings(config);
      originalConfig = JSON.parse(JSON.stringify(config));
      onClose();
    } catch (err) {
      console.error('Failed to save settings:', err);
      alert('Failed to save settings: ' + (err instanceof Error ? err.message : String(err)));
    }
  }

  function handleCancel() {
    if (originalConfig) {
      config = JSON.parse(JSON.stringify(originalConfig));
    }
    onClose();
  }

  async function handleBrowseVault() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Vault Folder'
      });

      if (selected && typeof selected === 'string' && config) {
        config.vault.path = selected;
      }
    } catch (err) {
      console.error('Failed to select folder:', err);
    }
  }

  async function handleTestConnection() {
    testStatus = 'testing';
    testError = '';

    try {
      const result = await aiTestConnection();
      testStatus = 'success';
      setTimeout(() => {
        testStatus = 'idle';
      }, 3000);
    } catch (err) {
      testStatus = 'error';
      testError = err instanceof Error ? err.message : String(err);
    }
  }

  async function handleReindex() {
    showReindexModal = true;
  }

  async function confirmReindex() {
    isReindexing = true;
    try {
      await invoke('reindex_vault');
    } catch (err) {
      alert('Failed to reindex: ' + (err instanceof Error ? err.message : String(err)));
    } finally {
      isReindexing = false;
    }
  }

  function getMaskedApiKey(key: string): string {
    if (!key || key.length < 8) return key;
    return 'sk-...' + key.slice(-4);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleCancel();
    }
  }

  function addAgent() {
    if (!config || !newAgentName.trim()) return;

    const newAgent = {
      id: crypto.randomUUID(),
      name: newAgentName.trim(),
      base_url: 'https://api.openai.com/v1',
      api_key: '',
      models: [],
      embedding_model: '',
      embedding_base_url: null,
      embedding_api_key: null
    };

    config.agents = [...config.agents, newAgent];
    if (!config.active_agent_id) {
      config.active_agent_id = newAgent.id;
    }
    newAgentName = '';
  }

  function deleteAgent(id: string) {
    if (!config) return;

    config.agents = config.agents.filter(a => a.id !== id);
    if (config.active_agent_id === id) {
      config.active_agent_id = config.agents[0]?.id || null;
    }
  }

  function setActiveAgent(id: string) {
    if (!config) return;
    config.active_agent_id = id;
  }

  function addModelToAgent(agentIndex: number) {
    if (!config || !newModelInput.trim()) return;

    const agent = config.agents[agentIndex];
    if (!agent.models.includes(newModelInput.trim())) {
      agent.models = [...agent.models, newModelInput.trim()];
    }
    newModelInput = '';
  }

  function removeModelFromAgent(agentIndex: number, model: string) {
    if (!config) return;

    const agent = config.agents[agentIndex];
    agent.models = agent.models.filter(m => m !== model);
  }
</script>

{#if isOpen && config}
  <div class="settings-overlay" onclick={handleCancel} onkeydown={handleKeyDown} role="button" tabindex="-1">
    <div class="settings-modal" onclick={(e) => e.stopPropagation()} role="dialog">
      <div class="settings-header">
        <h2>Settings</h2>
        <button class="close-btn" onclick={handleCancel} title="Close (Esc)">
          <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
            <path d="M5 5L15 15M15 5L5 15" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <div class="settings-content">
        <!-- Vault Section -->
        <section class="settings-section">
          <h3 class="section-header" onclick={() => toggleSection('vault')}>
            <svg class="chevron" class:expanded={expandedSections.vault} width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M6 4L10 8L6 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            Vault
          </h3>

          {#if expandedSections.vault}
          <div class="section-content">
          <div class="field">
            <label for="vault-path">Vault Path</label>
            <div class="input-with-button">
              <input
                id="vault-path"
                type="text"
                bind:value={config.vault.path}
                placeholder="/path/to/vault"
                readonly
              />
              <button class="browse-btn" onclick={handleBrowseVault}>Browse</button>
            </div>
          </div>

          <div class="field">
            <button
              class="secondary-btn"
              onclick={handleReindex}
              disabled={isReindexing}
            >
              {isReindexing ? 'Reindexing...' : 'Reindex Vault'}
            </button>
          </div>
          </div>
          {/if}
        </section>

        <!-- AI Providers Section -->
        <section class="settings-section">
          <h3 class="section-header" onclick={() => toggleSection('ai')}>
            <svg class="chevron" class:expanded={expandedSections.ai} width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M6 4L10 8L6 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            AI Providers
          </h3>

          {#if expandedSections.ai}
          <div class="section-content">
            <!-- Add new agent -->
            <div class="field">
              <div class="input-with-button">
                <input
                  type="text"
                  bind:value={newAgentName}
                  placeholder="Provider name..."
                  onkeydown={(e) => e.key === 'Enter' && addAgent()}
                />
                <button class="browse-btn" onclick={addAgent}>+ Add Provider</button>
              </div>
            </div>

            <!-- List of agents -->
            {#each config.agents as agent, index (agent.id)}
              <div class="agent-card" class:active={config.active_agent_id === agent.id}>
                <div class="agent-header">
                  <div class="agent-title">
                    <input
                      type="text"
                      bind:value={agent.name}
                      class="agent-name-input"
                      placeholder="Provider name"
                    />
                    {#if config.active_agent_id === agent.id}
                      <span class="active-badge">Active</span>
                    {/if}
                  </div>
                  <div class="agent-actions">
                    {#if config.active_agent_id !== agent.id}
                      <button
                        class="icon-btn-small"
                        onclick={() => setActiveAgent(agent.id)}
                        title="Set as active"
                      >
                        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                          <circle cx="7" cy="7" r="5.5" stroke="currentColor" stroke-width="1.2"/>
                        </svg>
                      </button>
                    {/if}
                    <button
                      class="icon-btn-small delete"
                      onclick={() => deleteAgent(agent.id)}
                      title="Delete provider"
                      disabled={config.agents.length === 1}
                    >
                      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                        <path d="M3 3L11 11M11 3L3 11" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
                      </svg>
                    </button>
                  </div>
                </div>

                <div class="agent-fields">
                  <div class="field">
                    <label>Base URL</label>
                    <input
                      type="text"
                      bind:value={agent.base_url}
                      placeholder="https://api.openai.com/v1"
                    />
                  </div>

                  <div class="field">
                    <label>API Key</label>
                    <div class="input-with-button">
                      <input
                        type={showApiKey ? 'text' : 'password'}
                        bind:value={agent.api_key}
                        placeholder="sk-..."
                      />
                      <button
                        class="icon-btn"
                        onclick={() => showApiKey = !showApiKey}
                        title={showApiKey ? 'Hide' : 'Show'}
                      >
                        {#if showApiKey}
                          <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
                            <path d="M2 9C2 9 4.5 4 9 4C13.5 4 16 9 16 9C16 9 13.5 14 9 14C4.5 14 2 9 2 9Z" stroke="currentColor" stroke-width="1.3"/>
                            <circle cx="9" cy="9" r="2" stroke="currentColor" stroke-width="1.3"/>
                            <path d="M2 2L16 16" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
                          </svg>
                        {:else}
                          <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
                            <path d="M2 9C2 9 4.5 4 9 4C13.5 4 16 9 16 9C16 9 13.5 14 9 14C4.5 14 2 9 2 9Z" stroke="currentColor" stroke-width="1.3"/>
                            <circle cx="9" cy="9" r="2" stroke="currentColor" stroke-width="1.3"/>
                          </svg>
                        {/if}
                      </button>
                    </div>
                  </div>

                  <div class="field">
                    <label>Chat Models</label>
                    <div class="models-list">
                      {#each agent.models as model}
                        <div class="model-chip">
                          <span>{model}</span>
                          <button
                            class="chip-delete"
                            onclick={() => removeModelFromAgent(index, model)}
                            disabled={agent.models.length === 1}
                          >
                            <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
                              <path d="M2 2L8 8M8 2L2 8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
                            </svg>
                          </button>
                        </div>
                      {/each}
                    </div>
                    <div class="input-with-button">
                      <input
                        type="text"
                        bind:value={newModelInput}
                        placeholder="gpt-4o"
                        onkeydown={(e) => e.key === 'Enter' && addModelToAgent(index)}
                      />
                      <button class="browse-btn" onclick={() => addModelToAgent(index)}>+ Add</button>
                    </div>
                  </div>

                  <div class="field">
                    <label>Embedding Model</label>
                    <input
                      type="text"
                      bind:value={agent.embedding_model}
                      placeholder="e.g., text-embedding-3-small, bge-large-en-v1.5"
                    />
                  </div>

                  <div class="subsection-header">
                    <span class="subsection-title">Separate Embedding Provider (Optional)</span>
                    <span class="subsection-hint">Leave empty to use main provider</span>
                  </div>

                  <div class="field">
                    <label>Embedding Base URL</label>
                    <input
                      type="text"
                      bind:value={agent.embedding_base_url}
                      placeholder="https://api.openai.com/v1"
                    />
                  </div>

                  <div class="field">
                    <label>Embedding API Key</label>
                    <input
                      type="password"
                      bind:value={agent.embedding_api_key}
                      placeholder="sk-..."
                    />
                  </div>
                </div>
              </div>
            {/each}
          </div>
          {/if}
        </section>

        <!-- Vision OCR Section -->
        <section class="settings-section">
          <h3 class="section-header" onclick={() => toggleSection('vision')}>
            <svg class="chevron" class:expanded={expandedSections.vision} width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M6 4L10 8L6 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            Vision OCR
          </h3>

          {#if expandedSections.vision}
          <div class="section-content">
            <div class="field">
              <label class="checkbox-label">
                <input
                  type="checkbox"
                  bind:checked={config.vision.enabled}
                />
                <span>Enable Vision OCR for scans and image-heavy documents</span>
              </label>
              <p class="field-hint">Uses AI vision models to extract text from scanned PDFs and slides. ~$0.01 per page.</p>
            </div>

            <div class="field">
              <label for="vision-model">Vision Model</label>
              <input
                id="vision-model"
                type="text"
                bind:value={config.vision.model}
                placeholder="openai/gpt-4o-mini"
              />
              <p class="field-hint">Model for vision OCR (default: openai/gpt-4o-mini)</p>
            </div>

            <div class="field">
              <label for="vision-base-url">Vision API Base URL (optional)</label>
              <input
                id="vision-base-url"
                type="text"
                bind:value={config.vision.base_url}
                placeholder="Same as main AI"
              />
              <p class="field-hint">Leave empty to use the same API as main AI provider</p>
            </div>

            <div class="field">
              <label for="vision-api-key">Vision API Key (optional)</label>
              <div class="input-with-button">
                <input
                  id="vision-api-key"
                  type={showVisionApiKey ? 'text' : 'password'}
                  bind:value={config.vision.api_key}
                  placeholder="Same as main AI"
                />
                <button
                  class="browse-btn"
                  onclick={() => showVisionApiKey = !showVisionApiKey}
                  type="button"
                >
                  {showVisionApiKey ? 'Hide' : 'Show'}
                </button>
              </div>
              <p class="field-hint">Leave empty to use the same API key as main AI provider</p>
            </div>
          </div>
          {/if}
        </section>

        <!-- Editor Section -->
        <section class="settings-section">
          <h3 class="section-header" onclick={() => toggleSection('editor')}>
            <svg class="chevron" class:expanded={expandedSections.editor} width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M6 4L10 8L6 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            Editor
          </h3>

          {#if expandedSections.editor}
          <div class="section-content">
          <div class="field">
            <label for="font-family">Font Family</label>
            <input
              id="font-family"
              type="text"
              bind:value={config.editor.font_family}
              placeholder="JetBrains Mono"
            />
          </div>

          <div class="field">
            <label for="font-size">Font Size (px)</label>
            <div class="number-input">
              <input
                id="font-size"
                type="number"
                min="10"
                max="24"
                step="1"
                bind:value={config.editor.font_size}
              />
              <div class="number-controls">
                <button
                  class="number-btn"
                  onclick={() => incrementNumber('font_size', 1)}
                  type="button"
                >
                  <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
                    <path d="M5 2L8 6H2L5 2Z" fill="currentColor"/>
                  </svg>
                </button>
                <button
                  class="number-btn"
                  onclick={() => incrementNumber('font_size', -1)}
                  type="button"
                >
                  <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
                    <path d="M5 8L2 4H8L5 8Z" fill="currentColor"/>
                  </svg>
                </button>
              </div>
            </div>
          </div>

          <div class="field">
            <label for="autosave">Autosave Delay (ms)</label>
            <div class="number-input">
              <input
                id="autosave"
                type="number"
                min="500"
                max="10000"
                step="100"
                bind:value={config.editor.autosave_ms}
              />
              <div class="number-controls">
                <button
                  class="number-btn"
                  onclick={() => incrementNumber('autosave_ms', 100)}
                  type="button"
                >
                  <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
                    <path d="M5 2L8 6H2L5 2Z" fill="currentColor"/>
                  </svg>
                </button>
                <button
                  class="number-btn"
                  onclick={() => incrementNumber('autosave_ms', -100)}
                  type="button"
                >
                  <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
                    <path d="M5 8L2 4H8L5 8Z" fill="currentColor"/>
                  </svg>
                </button>
              </div>
            </div>
          </div>
          </div>
          {/if}
        </section>
      </div>

      <div class="settings-footer">
        <button class="btn-secondary" onclick={handleCancel}>Cancel</button>
        <button class="btn-primary" onclick={handleSave}>Save</button>
      </div>
    </div>
  </div>

  <ConfirmModal
    bind:isOpen={showReindexModal}
    title="Reindex Vault"
    message="Reindex entire vault? This may take a while for large vaults."
    confirmText="Reindex"
    onConfirm={confirmReindex}
  />
{/if}

<style>
  .settings-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fadeIn 0.2s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .settings-modal {
    width: 600px;
    max-height: 80vh;
    background: rgba(26, 27, 38, 0.7);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border-radius: 8px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
    animation: slideUp 0.2s ease-out;
  }

  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .settings-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px;
    border-bottom: 1px solid var(--border);
  }

  .settings-header h2 {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    transition: all 0.2s;
  }

  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .settings-content {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
  }

  /* Hide scrollbar in settings */
  .settings-content::-webkit-scrollbar {
    display: none;
  }

  .settings-content {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }

  .settings-section {
    margin-bottom: 32px;
  }

  .settings-section:last-child {
    margin-bottom: 0;
  }

  .settings-section h3 {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 16px 0;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    user-select: none;
    transition: color 0.15s ease;
  }

  .section-header:hover {
    color: var(--accent);
  }

  .chevron {
    flex-shrink: 0;
    transition: transform 0.2s ease;
    transform: rotate(0deg);
  }

  .chevron.expanded {
    transform: rotate(90deg);
  }

  .section-content {
    animation: expandSection 0.2s ease-out;
  }

  @keyframes expandSection {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .field {
    margin-bottom: 16px;
  }

  .field:last-child {
    margin-bottom: 0;
  }

  label {
    display: block;
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  input[type="text"],
  input[type="password"],
  input[type="number"],
  select {
    width: 100%;
    padding: 8px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 0.875rem;
    transition: border-color 0.2s;
  }

  input[type="text"]:focus,
  input[type="password"]:focus,
  input[type="number"]:focus,
  select:focus {
    outline: none;
    border-color: var(--accent);
  }

  input[type="text"]:disabled,
  input[type="password"]:disabled,
  input[type="number"]:disabled,
  select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  input[type="text"]::placeholder,
  input[type="password"]::placeholder {
    color: var(--text-muted);
  }

  /* Hide default number spinners */
  input[type="number"]::-webkit-inner-spin-button,
  input[type="number"]::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  input[type="number"] {
    -moz-appearance: textfield;
  }

  /* Custom number input with controls */
  .number-input {
    position: relative;
    display: flex;
  }

  .number-input input {
    flex: 1;
    padding-right: 32px;
  }

  .number-controls {
    position: absolute;
    right: 1px;
    top: 1px;
    bottom: 1px;
    display: flex;
    flex-direction: column;
    width: 24px;
    background: var(--bg-secondary);
    border-left: 1px solid var(--border);
    border-radius: 0 3px 3px 0;
  }

  .number-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    margin: 0;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .number-btn:first-child {
    border-bottom: 1px solid var(--border);
  }

  .number-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .number-btn:active {
    background: var(--accent);
    color: #fff;
  }

  input[type="range"] {
    width: 100%;
    margin: 8px 0;
  }

  .range-value {
    text-align: right;
    font-size: 0.8125rem;
    color: var(--text-secondary);
    margin-top: 4px;
  }

  .input-with-button {
    display: flex;
    gap: 8px;
  }

  .input-with-button input {
    flex: 1;
  }

  .browse-btn,
  .icon-btn {
    padding: 8px 14px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-secondary);
    font-size: 0.8125rem;
    cursor: pointer;
    transition: all 0.2s;
    white-space: nowrap;
  }

  .icon-btn {
    padding: 8px 10px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .browse-btn:hover,
  .icon-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .field-hint {
    margin-top: 4px;
    font-size: 0.75rem;
    color: var(--text-muted);
    font-family: 'JetBrains Mono', monospace;
  }

  .secondary-btn {
    padding: 8px 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-secondary);
    font-size: 0.8125rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .secondary-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .secondary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error-text {
    margin-top: 6px;
    font-size: 0.8125rem;
    color: var(--error);
  }

  .settings-footer {
    display: flex;
    gap: 12px;
    justify-content: flex-end;
    padding: 16px 24px;
    border-top: 1px solid var(--border);
  }

  .btn-secondary,
  .btn-primary {
    padding: 10px 20px;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-secondary {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .btn-secondary:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .btn-primary {
    background: var(--accent);
    color: var(--bg-primary);
  }

  .btn-primary:hover {
    background: var(--accent-hover);
  }

  /* AI Agent Cards */
  .agent-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 16px;
    margin-bottom: 16px;
    transition: all 0.2s ease;
  }

  .agent-card.active {
    border-color: var(--accent);
    background: rgba(122, 162, 247, 0.05);
  }

  .agent-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border);
  }

  .agent-title {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
  }

  .agent-name-input {
    background: transparent !important;
    border: none !important;
    padding: 0 !important;
    font-size: 0.9375rem !important;
    font-weight: 600 !important;
    color: var(--text-primary) !important;
    width: auto !important;
    min-width: 120px;
  }

  .agent-name-input:focus {
    outline: none !important;
    border-bottom: 1px solid var(--accent) !important;
  }

  .active-badge {
    padding: 2px 8px;
    background: var(--accent);
    color: var(--bg-primary);
    font-size: 0.6875rem;
    font-weight: 600;
    border-radius: 3px;
    text-transform: uppercase;
  }

  .agent-actions {
    display: flex;
    gap: 4px;
  }

  .icon-btn-small {
    width: 24px;
    height: 24px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .icon-btn-small:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .icon-btn-small.delete:hover:not(:disabled) {
    background: rgba(247, 118, 142, 0.15);
    border-color: var(--error);
    color: var(--error);
  }

  .icon-btn-small:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .agent-fields {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .models-list {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 8px;
    min-height: 28px;
  }

  .model-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 0.75rem;
    color: var(--text-primary);
  }

  .model-chip span {
    user-select: none;
  }

  .chip-delete {
    padding: 0;
    margin: 0;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.15s ease;
  }

  .chip-delete:hover:not(:disabled) {
    color: var(--error);
  }

  .chip-delete:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .subsection-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: 16px 0 8px 0;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .subsection-title {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .subsection-hint {
    font-size: 0.6875rem;
    color: var(--text-muted);
  }
</style>
