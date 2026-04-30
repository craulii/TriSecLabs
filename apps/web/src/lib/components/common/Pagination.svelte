<script lang="ts">
  let {
    page,
    limit,
    total,
    onchange,
  }: {
    page:     number;
    limit:    number;
    total:    number;
    onchange: (page: number) => void;
  } = $props();

  const totalPages = $derived(Math.ceil(total / limit) || 1);
  const from       = $derived((page - 1) * limit + 1);
  const to         = $derived(Math.min(page * limit, total));
</script>

<div class="flex items-center justify-between px-1 py-2 text-sm"
     style="color: var(--text-secondary);">
  <span>{from}–{to} de {total}</span>

  <div class="flex items-center gap-1">
    <button
      class="px-2 py-1 rounded disabled:opacity-30"
      style="background: var(--bg-elevated);"
      disabled={page <= 1}
      on:click={() => onchange(page - 1)}
    >
      ‹
    </button>

    {#each Array(Math.min(totalPages, 7)) as _, i}
      {@const p = i + Math.max(1, page - 3)}
      {#if p <= totalPages}
        <button
          class="px-2 py-1 rounded min-w-7 text-center"
          style="
            background: {p === page ? 'var(--accent)' : 'var(--bg-elevated)'};
            color: {p === page ? 'white' : 'var(--text-secondary)'};
          "
          on:click={() => onchange(p)}
        >
          {p}
        </button>
      {/if}
    {/each}

    <button
      class="px-2 py-1 rounded disabled:opacity-30"
      style="background: var(--bg-elevated);"
      disabled={page >= totalPages}
      on:click={() => onchange(page + 1)}
    >
      ›
    </button>
  </div>
</div>
