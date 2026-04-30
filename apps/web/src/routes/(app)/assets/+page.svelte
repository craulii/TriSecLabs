<script lang="ts">
  import { onMount }     from 'svelte';
  import { targetsApi }  from '$lib/api/targets';
  import { toasts }      from '$lib/stores/ui';
  import { ApiError }    from '$lib/api/client';
  import DataTable       from '$lib/components/common/DataTable.svelte';
  import RiskBadge       from '$lib/components/common/RiskBadge.svelte';
  import type { ScanTarget } from '$lib/types/models';

  let targets = $state<ScanTarget[]>([]);
  let loading = $state(true);

  const columns = [
    { key: 'name',            label: 'Nombre',      sortable: true },
    { key: 'kind',            label: 'Tipo',        sortable: true, width: '100px' },
    { key: 'value',           label: 'Valor',       sortable: true },
    { key: 'risk_level',      label: 'Riesgo',      width: '110px', align: 'center' as const },
    { key: 'last_scanned_at', label: 'Último scan', sortable: true, width: '140px' },
  ];

  onMount(async () => {
    try {
      targets = await targetsApi.list();
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error cargando assets');
    } finally {
      loading = false;
    }
  });

  async function enqueueScan(id: string) {
    try {
      await targetsApi.enqueueScan(id);
      toasts.success('Scan encolado correctamente');
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error al encolar scan');
    }
  }

  async function enqueueLlm(id: string) {
    try {
      await targetsApi.enqueueLlmReport(id);
      toasts.warning('Informe LLM encolado. Requiere servidor LLM activo para procesarse.');
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error al encolar informe');
    }
  }

  function fmtDate(iso: string | null) {
    if (!iso) return '—';
    return new Date(iso).toLocaleDateString('es', { day: '2-digit', month: 'short', year: 'numeric' });
  }
</script>

<svelte:head><title>Assets — TriSecLabs</title></svelte:head>

<div class="rounded-xl overflow-hidden"
     style="background: var(--bg-surface); border: 1px solid var(--border);">
  <div class="px-4 py-3" style="border-bottom: 1px solid var(--border);">
    <h1 class="text-sm font-semibold" style="color: var(--text-primary);">Assets</h1>
    <p class="text-xs mt-0.5" style="color: var(--text-muted);">
      {targets.length} activo{targets.length !== 1 ? 's' : ''}
    </p>
  </div>

  <DataTable rows={targets} {columns} {loading} emptyLabel="Sin assets configurados">
    {#snippet cell({ row, col })}
      {#if col.key === 'name'}
        <a href="/assets/{row.id}" class="font-medium hover:underline"
           style="color: var(--text-primary);">{row.name}</a>
      {:else if col.key === 'risk_level'}
        {#if row.risk_level}
          <RiskBadge value={row.risk_level} />
        {:else}
          <span style="color: var(--text-muted);">—</span>
        {/if}
      {:else if col.key === 'last_scanned_at'}
        <span style="color: var(--text-secondary); font-size: 12px;">{fmtDate(row.last_scanned_at)}</span>
      {:else}
        <span style="color: var(--text-secondary);">{(row as unknown as Record<string, unknown>)[col.key] ?? '—'}</span>
      {/if}
    {/snippet}

    {#snippet rowActions({ row })}
      <div class="flex items-center gap-1">
        <button onclick={() => enqueueScan(row.id)}
                class="px-2 py-1 rounded text-xs whitespace-nowrap"
                style="background: var(--bg-elevated); color: var(--text-secondary);"
                title="Lanzar scan nmap">
          Scan
        </button>
        <button onclick={() => enqueueLlm(row.id)}
                class="px-2 py-1 rounded text-xs whitespace-nowrap"
                style="background: var(--bg-elevated); color: var(--text-secondary);"
                title="Generar informe LLM">
          LLM
        </button>
      </div>
    {/snippet}
  </DataTable>
</div>
