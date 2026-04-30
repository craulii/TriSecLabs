<script lang="ts">
  import '../app.css';
  import { onMount }         from 'svelte';
  import { theme, toasts }   from '$lib/stores/ui';
  import type { Toast }      from '$lib/stores/ui';

  let { children } = $props();

  onMount(() => {
    const unsub = theme.subscribe(t => {
      document.documentElement.setAttribute('data-theme', t);
    });
    return unsub;
  });

  const toastColors: Record<Toast['kind'], string> = {
    info:    'var(--accent)',
    success: '#22c55e',
    warning: '#f59e0b',
    error:   '#ef4444',
  };
</script>

{@render children()}

<!-- Toast stack -->
{#if $toasts.length > 0}
  <div class="fixed bottom-5 right-5 flex flex-col gap-2 z-50" style="max-width: 340px;">
    {#each $toasts as toast (toast.id)}
      <div
        class="flex items-start gap-3 px-4 py-3 rounded-lg text-sm shadow-lg"
        style="
          background: var(--bg-elevated);
          border: 1px solid {toastColors[toast.kind]};
          color: var(--text-primary);
        "
      >
        <span style="color: {toastColors[toast.kind]}; flex-shrink: 0; margin-top: 1px;">
          {#if toast.kind === 'success'}✓{:else if toast.kind === 'error'}✕{:else if toast.kind === 'warning'}⚠{:else}ℹ{/if}
        </span>
        <span class="flex-1">{toast.message}</span>
        <button
          onclick={() => toasts.dismiss(toast.id)}
          style="color: var(--text-muted); flex-shrink: 0;"
          class="leading-none"
        >×</button>
      </div>
    {/each}
  </div>
{/if}
