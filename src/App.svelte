<script lang="ts">
  import { onMount } from 'svelte';
  import Layout from './lib/components/Layout.svelte';
  import { notebooksStore } from './lib/stores/notebooks.svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { vaultGetPath, vaultInit } from './lib/utils/tauri';
  import { withTimeout } from './lib/utils/async';

  let isInitialized = $state(false);
  let isInitializing = $state(true);
  let isPickingVault = $state(false);
  let initError = $state<string | null>(null);

  onMount(async () => {
    try {
      // Try to get existing vault path
      await vaultGetPath();
      await notebooksStore.loadNotebooks();
      isInitialized = true;
    } catch (err) {
      // No vault configured (or backend unavailable) — show setup UI instead of blocking on a dialog.
      const message = err instanceof Error ? err.message : String(err);
      initError = message.includes('No vault initialized') ? null : message;
    } finally {
      isInitializing = false;
    }
  });

  async function promptForVault() {
    if (isPickingVault) return;
    isPickingVault = true;
    initError = null;

    let selected: string | string[] | null = null;
    try {
      selected = await withTimeout(
        open({
          directory: true,
          multiple: false,
          title: 'Select or create a vault directory',
        }) as Promise<string | string[] | null>,
        60_000,
        'Vault picker timed out'
      );
    } catch (err) {
      initError = err instanceof Error ? err.message : String(err);
      console.error('Vault picker failed:', err);
      return;
    } finally {
      isPickingVault = false;
    }

    if (Array.isArray(selected)) {
      selected = selected[0] ?? null;
    }

    if (selected && typeof selected === 'string') {
      try {
        isInitializing = true;
        await vaultInit(selected);
        await notebooksStore.loadNotebooks();
        isInitialized = true;
      } catch (err) {
        console.error('Failed to initialize vault:', err);
        initError = err instanceof Error ? err.message : String(err);
      } finally {
        isInitializing = false;
      }
    }
  }
</script>

{#if isInitializing}
  <div class="loading">
    <p>Loading...</p>
  </div>
{:else if isInitialized}
  <Layout />
{:else}
  <div class="setup">
    <div class="setup-content">
      <h1>Welcome to NoteForge</h1>
      <p>Select a directory for your notes vault</p>
      {#if initError}
        <p class="error">{initError}</p>
      {/if}
      <button onclick={promptForVault} disabled={isPickingVault}>
        {isPickingVault ? 'Opening…' : 'Select Vault Directory'}
      </button>
    </div>
  </div>
{/if}

<style>
  .loading,
  .setup {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100vw;
    height: 100vh;
    background-color: var(--bg-primary);
    color: var(--text-primary);
  }

  .setup-content {
    text-align: center;
  }

  .setup-content h1 {
    font-size: 2rem;
    margin-bottom: 1rem;
    color: var(--accent);
  }

  .setup-content p {
    font-size: 1rem;
    margin-bottom: 2rem;
    color: var(--text-secondary);
  }

  .setup-content .error {
    margin: 0.75rem 0 1rem;
    color: var(--error);
    max-width: 520px;
    word-break: break-word;
  }

  .setup-content button {
    padding: 0.75rem 1.5rem;
    font-size: 1rem;
    background-color: var(--accent);
    color: var(--bg-primary);
    border-radius: 6px;
    font-weight: 600;
  }

  .setup-content button:hover {
    background-color: var(--accent-hover);
  }

  .setup-content button:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }
</style>
