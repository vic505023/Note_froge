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

  /* Tabs */
  .tabs {
    display: flex;
    width: 100%;
    border-bottom: 1px solid var(--border);
  }

  .tab {
    flex: 1;
    padding: 10px 0;
    background: transparent;
    border: none;
    font-size: 12px;
    font-weight: 400;
    color: var(--text-secondary);
    cursor: pointer;
    text-align: center;
    position: relative;
    transition: all 0.15s ease;
  }

  .tab:hover {
    color: var(--text-primary);
  }

  .tab.active {
    color: var(--text-primary);
    font-weight: 600;
  }

  .tab.active::after {
    content: '';
    position: absolute;
    bottom: -1px;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--accent);
  }

  /* Content */
  .panel-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
