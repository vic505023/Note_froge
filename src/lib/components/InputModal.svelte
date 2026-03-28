<script lang="ts">
  let {
    isOpen = $bindable(false),
    title = 'Enter name',
    placeholder = '',
    onSubmit
  }: {
    isOpen: boolean;
    title: string;
    placeholder?: string;
    onSubmit: (value: string) => void;
  } = $props();

  let inputValue = $state('');
  let inputElement: HTMLInputElement;

  $effect(() => {
    if (isOpen) {
      inputValue = '';
      setTimeout(() => inputElement?.focus(), 0);
    }
  });

  function handleSubmit() {
    if (!inputValue.trim()) return;
    onSubmit(inputValue.trim());
    isOpen = false;
  }

  function handleCancel() {
    isOpen = false;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleSubmit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      handleCancel();
    }
  }

  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleCancel();
    }
  }
</script>

{#if isOpen}
  <div class="modal-overlay" onclick={handleOverlayClick}>
    <div class="modal-content">
      <h2 class="modal-title">{title}</h2>

      <input
        bind:this={inputElement}
        bind:value={inputValue}
        onkeydown={handleKeyDown}
        type="text"
        placeholder={placeholder}
        class="modal-input"
      />

      <div class="modal-actions">
        <button class="modal-btn cancel-btn" onclick={handleCancel}>
          Cancel
        </button>
        <button
          class="modal-btn submit-btn"
          onclick={handleSubmit}
          disabled={!inputValue.trim()}
        >
          Create
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
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
  }

  .modal-content {
    background: rgba(22, 27, 34, 0.85);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 24px;
    width: 400px;
    max-width: 90vw;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .modal-title {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 16px;
  }

  .modal-input {
    width: 100%;
    padding: 10px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 0.875rem;
    font-family: inherit;
    transition: border-color 0.15s ease;
  }

  .modal-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .modal-input::placeholder {
    color: var(--text-muted);
  }

  .modal-actions {
    display: flex;
    gap: 8px;
    margin-top: 20px;
    justify-content: flex-end;
  }

  .modal-btn {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    border: none;
  }

  .cancel-btn {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .cancel-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    border-color: var(--error);
  }

  .cancel-btn:active {
    background: var(--error);
    color: #fff;
  }

  .submit-btn {
    background: var(--accent);
    color: #fff;
  }

  .submit-btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .submit-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
