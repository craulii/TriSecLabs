<script lang="ts">
  import { sidebarCollapsed, theme } from '$lib/stores/ui';
  import { auth }                    from '$lib/stores/auth';

  let { title = '' }: { title?: string } = $props();
</script>

<header
  class="flex items-center justify-between px-5 h-14 flex-shrink-0"
  style="
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border);
  "
>
  <!-- Izquierda: toggle sidebar + título de página -->
  <div class="flex items-center gap-4">
    <button
      on:click={() => sidebarCollapsed.update(v => !v)}
      class="p-1.5 rounded transition-colors"
      style="color: var(--text-secondary);"
      title="Toggle sidebar"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none"
           stroke="currentColor" stroke-width="2">
        <line x1="3" y1="6"  x2="21" y2="6"  />
        <line x1="3" y1="12" x2="21" y2="12" />
        <line x1="3" y1="18" x2="21" y2="18" />
      </svg>
    </button>
    {#if title}
      <h1 class="font-semibold text-base" style="color: var(--text-primary);">
        {title}
      </h1>
    {/if}
  </div>

  <!-- Derecha: tema + info de sesión -->
  <div class="flex items-center gap-3">
    <!-- Toggle dark/light -->
    <button
      on:click={theme.toggle}
      class="p-1.5 rounded transition-colors"
      style="color: var(--text-secondary);"
      title="Cambiar tema"
    >
      {#if $theme === 'dark'}
        <!-- Sun icon -->
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none"
             stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="5"/>
          <line x1="12" y1="1" x2="12" y2="3"/>  <line x1="12" y1="21" x2="12" y2="23"/>
          <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/>
          <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/>
          <line x1="1" y1="12" x2="3" y2="12"/>  <line x1="21" y1="12" x2="23" y2="12"/>
          <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/>
          <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
        </svg>
      {:else}
        <!-- Moon icon -->
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none"
             stroke="currentColor" stroke-width="2">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
        </svg>
      {/if}
    </button>

    <!-- Indicador de rol/tenant -->
    <span class="text-xs px-2 py-1 rounded"
          style="background: var(--bg-elevated); color: var(--text-secondary);">
      {$auth.role}
    </span>
  </div>
</header>
