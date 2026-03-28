<script lang="ts">
  let {
    isOpen = $bindable(false),
    title = 'Confirm',
    message = 'Are you sure?',
    confirmText = 'Confirm',
    cancelText = 'Cancel',
    onConfirm
  }: {
    isOpen: boolean;
    title: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    onConfirm: () => void;
  } = $props();

  function handleConfirm() {
    onConfirm();
    isOpen = false;
  }

  function handleCancel() {
    isOpen = false;
  }

  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleCancel();
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleCancel();
    } else if (e.key === 'Enter') {
      handleConfirm();
    }
  }

  $effect(() => {
    if (isOpen) {
      window.addEventListener('keydown', handleKeyDown);
      return () => window.removeEventListener('keydown', handleKeyDown);
    }
  });
</script>

{#if isOpen}
  <div class="modal-overlay" onclick={handleOverlayClick}>
    <div class="modal-content">
      <h2 class="modal-title">{title}</h2>
      <p class="modal-message">{message}</p>

      <div class="modal-actions">
        <button class="modal-btn cancel-btn" onclick={handleCancel}>
          {cancelText}
        </button>
        <button class="modal-btn confirm-btn" onclick={handleConfirm}>
          {confirmText}
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
    z-index: 9999;
  }

  .modal-content {
    background: rgba(22, 27, 34, 0.85);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 24px;
    width: 360px;
    max-width: 90vw;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .modal-title {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 12px;
  }

  .modal-message {
    font-size: 0.875rem;
    color: var(--text-secondary);
    line-height: 1.5;
    margin-bottom: 20px;
    white-space: pre-line;
  }

  .modal-actions {
    display: flex;
    gap: 8px;
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

  .confirm-btn {
    background: var(--error);
    color: #fff;
    border: 1px solid var(--error);
  }

  .confirm-btn:hover {
    background: #c9515a;
  }

  .confirm-btn:active {
    background: #d62839;
  }
</style>
