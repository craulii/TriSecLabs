<script lang="ts" generics="T extends object">
  import type { Snippet } from 'svelte';

  // ─── Props ──────────────────────────────────────────────────────────────────

  interface Column<R> {
    key:       keyof R | string;
    label:     string;
    sortable?: boolean;
    width?:    string;
    align?:    'left' | 'center' | 'right';
  }

  let {
    rows,
    columns,
    loading     = false,
    emptyLabel  = 'Sin resultados',
    rowKey      = 'id',
    // Slots
    cell,
    rowActions,
  }: {
    rows:         T[];
    columns:      Column<T>[];
    loading?:     boolean;
    emptyLabel?:  string;
    rowKey?:      string;
    cell?:        Snippet<[{ row: T; col: Column<T> }]>;
    rowActions?:  Snippet<[{ row: T }]>;
  } = $props();

  // ─── Sorting ─────────────────────────────────────────────────────────────────

  let sortKey   = $state<string | null>(null);
  let sortDir   = $state<'asc' | 'desc'>('asc');

  function toggleSort(key: string) {
    if (sortKey === key) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    } else {
      sortKey = key;
      sortDir = 'asc';
    }
  }

  const sorted = $derived(() => {
    if (!sortKey) return rows;
    return [...rows].sort((a, b) => {
      const av = a[sortKey as keyof T];
      const bv = b[sortKey as keyof T];
      const cmp = String(av ?? '').localeCompare(String(bv ?? ''), undefined, { numeric: true });
      return sortDir === 'asc' ? cmp : -cmp;
    });
  });
</script>

<div class="w-full overflow-x-auto">
  <table class="w-full border-collapse text-sm">
    <thead>
      <tr style="border-bottom: 1px solid var(--border);">
        {#each columns as col}
          <th
            class="px-3 py-2.5 text-left font-medium select-none"
            style="
              color: var(--text-secondary);
              width: {col.width ?? 'auto'};
              text-align: {col.align ?? 'left'};
              white-space: nowrap;
              {col.sortable ? 'cursor: pointer;' : ''}
            "
            on:click={() => col.sortable && toggleSort(String(col.key))}
          >
            <span class="inline-flex items-center gap-1">
              {col.label}
              {#if col.sortable && sortKey === String(col.key)}
                <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor">
                  {#if sortDir === 'asc'}
                    <polygon points="5,2 9,8 1,8"/>
                  {:else}
                    <polygon points="5,8 9,2 1,2"/>
                  {/if}
                </svg>
              {/if}
            </span>
          </th>
        {/each}
        {#if rowActions}
          <th class="px-3 py-2.5 w-10"></th>
        {/if}
      </tr>
    </thead>

    <tbody>
      {#if loading}
        {#each Array(5) as _, i}
          <tr style="border-bottom: 1px solid var(--border);">
            {#each columns as _}
              <td class="px-3 py-2.5">
                <div class="h-3 rounded animate-pulse" style="background: var(--bg-elevated); width: 80%;"></div>
              </td>
            {/each}
            {#if rowActions}
              <td class="px-3 py-2.5"></td>
            {/if}
          </tr>
        {/each}
      {:else if sorted().length === 0}
        <tr>
          <td
            colspan={columns.length + (rowActions ? 1 : 0)}
            class="px-3 py-10 text-center"
            style="color: var(--text-muted);"
          >
            {emptyLabel}
          </td>
        </tr>
      {:else}
        {#each sorted() as row ((row as unknown as Record<string, unknown>)[rowKey])}
          <tr
            class="transition-colors"
            style="border-bottom: 1px solid var(--border);"
            on:mouseenter={e => (e.currentTarget as HTMLElement).style.background = 'var(--bg-elevated)'}
            on:mouseleave={e => (e.currentTarget as HTMLElement).style.background = 'transparent'}
          >
            {#each columns as col}
              <td
                class="px-3 py-2.5"
                style="text-align: {col.align ?? 'left'}; white-space: nowrap;"
              >
                {#if cell}
                  {@render cell({ row, col })}
                {:else}
                  <span style="color: var(--text-primary);">
                    {row[col.key as keyof T] ?? '—'}
                  </span>
                {/if}
              </td>
            {/each}
            {#if rowActions}
              <td class="px-3 py-2.5">
                {@render rowActions({ row })}
              </td>
            {/if}
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>
</div>
