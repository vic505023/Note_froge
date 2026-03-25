<script lang="ts">
  import MarkdownIt from 'markdown-it';

  interface Props {
    role: 'user' | 'assistant' | 'system';
    content: string;
    isStreaming?: boolean;
  }

  let { role, content, isStreaming = false }: Props = $props();

  const md = new MarkdownIt('default', {
    html: false,
    breaks: true,
    linkify: true,
  }).enable(['table']);

  let renderedContent = $derived.by(() => {
    if (role === 'assistant') {
      try {
        const rendered = md.render(content);
        console.log('Rendered markdown length:', rendered.length, 'from content:', content.length);
        return rendered;
      } catch (e) {
        console.error('Markdown render error:', e);
        return content;
      }
    }
    return content;
  });
</script>

{#if role !== 'system'}
  <div class="message" class:user={role === 'user'} class:assistant={role === 'assistant'}>
    <div class="message-content">
      {#if role === 'assistant'}
        {@html renderedContent}
        {#if isStreaming}
          <span class="cursor">▊</span>
        {/if}
      {:else}
        {content}
      {/if}
    </div>
  </div>
{/if}

<style>
  .message {
    display: flex;
    margin-bottom: 14px;
  }

  .message.user {
    justify-content: flex-end;
  }

  .message.assistant {
    justify-content: flex-start;
  }

  .message-content {
    max-width: 85%;
    padding: 8px 12px;
    font-size: 12px;
    line-height: 1.6;
    word-wrap: break-word;
  }

  .message.user .message-content {
    background: var(--bg-elevated);
    color: var(--text-primary);
    border-radius: 12px 12px 2px 12px;
  }

  .message.assistant .message-content {
    background: transparent;
    color: var(--text-primary);
    border-left: 2px solid var(--accent);
    padding-left: 10px;
  }

  .message.assistant .message-content :global(p) {
    margin: 0 0 0.75em 0;
  }

  .message.assistant .message-content :global(p:last-child) {
    margin-bottom: 0;
  }

  .message.assistant .message-content :global(h1),
  .message.assistant .message-content :global(h2),
  .message.assistant .message-content :global(h3),
  .message.assistant .message-content :global(h4) {
    margin: 1.2em 0 0.6em 0;
    font-weight: 600;
    line-height: 1.3;
  }

  .message.assistant .message-content :global(h1:first-child),
  .message.assistant .message-content :global(h2:first-child),
  .message.assistant .message-content :global(h3:first-child),
  .message.assistant .message-content :global(h4:first-child) {
    margin-top: 0;
  }

  .message.assistant .message-content :global(h1) { font-size: 1.4em; }
  .message.assistant .message-content :global(h2) { font-size: 1.2em; }
  .message.assistant .message-content :global(h3) { font-size: 1.1em; }
  .message.assistant .message-content :global(h4) { font-size: 1em; }

  .message.assistant .message-content :global(code) {
    background: var(--bg-elevated);
    padding: 2px 6px;
    border-radius: 3px;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.85em;
  }

  .message.assistant .message-content :global(pre) {
    background: var(--bg-elevated);
    padding: 12px;
    border-radius: 6px;
    overflow-x: auto;
    margin: 1em 0;
  }

  .message.assistant .message-content :global(pre)::-webkit-scrollbar {
    height: 6px;
  }

  .message.assistant .message-content :global(pre)::-webkit-scrollbar-track {
    background: transparent;
  }

  .message.assistant .message-content :global(pre)::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 3px;
  }

  .message.assistant .message-content :global(pre)::-webkit-scrollbar-thumb:hover {
    background: var(--text-muted);
  }

  .message.assistant .message-content :global(pre code) {
    background: none;
    padding: 0;
  }

  .message.assistant .message-content :global(ul),
  .message.assistant .message-content :global(ol) {
    margin: 0.75em 0;
    padding-left: 1.8em;
  }

  .message.assistant .message-content :global(li) {
    margin: 0.35em 0;
  }

  .message.assistant .message-content :global(li > p) {
    margin: 0.25em 0;
  }

  .message.assistant .message-content :global(a) {
    color: var(--accent);
    text-decoration: none;
  }

  .message.assistant .message-content :global(a:hover) {
    text-decoration: underline;
  }

  .message.assistant .message-content :global(hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 1em 0;
  }

  .message.assistant .message-content :global(blockquote) {
    border-left: 3px solid var(--accent);
    padding-left: 1em;
    margin: 0.75em 0;
    color: var(--text-secondary);
  }

  .message.assistant .message-content :global(table) {
    border-collapse: collapse;
    width: 100%;
    margin: 1em 0;
    font-size: 11px;
  }

  .message.assistant .message-content :global(th),
  .message.assistant .message-content :global(td) {
    border: 1px solid var(--border);
    padding: 8px 12px;
    text-align: left;
  }

  .message.assistant .message-content :global(th) {
    background: var(--bg-elevated);
    font-weight: 600;
    color: var(--text-primary);
  }

  .message.assistant .message-content :global(tr:hover) {
    background: var(--bg-hover);
  }

  .message.assistant .message-content :global(thead tr:hover) {
    background: var(--bg-elevated);
  }

  .cursor {
    display: inline-block;
    margin-left: 2px;
    animation: blink 1s infinite;
  }

  @keyframes blink {
    0%, 50% { opacity: 1; }
    51%, 100% { opacity: 0; }
  }
</style>
