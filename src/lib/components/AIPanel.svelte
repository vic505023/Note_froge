<script lang="ts">
  import ChatTab from './ChatTab.svelte';
  import SearchTab from './SearchTab.svelte';

  let activeTab = $state<'chat' | 'search'>('chat');
</script>

<div class="ai-panel-container">
  <div class="tabs">
    <button
      class="tab"
      class:active={activeTab === 'chat'}
      onclick={() => activeTab = 'chat'}
    >
      Chat
    </button>
    <button
      class="tab"
      class:active={activeTab === 'search'}
      onclick={() => activeTab = 'search'}
    >
      Search
    </button>
  </div>

  <div class="panel-content">
    {#if activeTab === 'chat'}
      <ChatTab />
    {:else}
      <SearchTab />
    {/if}
  </div>
</div>

<style>
  .ai-panel-container {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  /* Tabs - macOS segmented control */
  .tabs {
    display: flex;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    padding: 2px;
    margin: 8px 10px;
  }

  .tab {
    flex: 1;
    padding: 5px 16px;
    background: transparent;
    border: none;
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    text-align: center;
    position: relative;
    transition: all 0.2s ease;
    border-radius: 6px;
  }

  .tab:hover:not(.active) {
    color: var(--text-primary);
  }

  .tab.active {
    color: var(--text-primary);
    background: var(--accent);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  }

  /* Content */
  .panel-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
