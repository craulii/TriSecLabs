<script lang="ts">
  import { onMount }        from 'svelte';
  import { targetsApi }     from '$lib/api/targets';
  import { metricsApi }     from '$lib/api/metrics';
  import { toasts }         from '$lib/stores/ui';
  import { ApiError }       from '$lib/api/client';
  import DataTable          from '$lib/components/common/DataTable.svelte';
  import RiskBadge          from '$lib/components/common/RiskBadge.svelte';
  import VulnTimeline       from '$lib/components/charts/VulnTimeline.svelte';
  import type { ScanTarget, Metric } from '$lib/types/models';

  let targets  = $state<ScanTarget[]>([]);
  let loading  = $state(true);
  let showForm = $state(false);

  // KPIs agregados desde los targets
  let kpis = $derived({
    total:    targets.length,
    avgRisk:  targets.length
      ? Math.round(targets.reduce((s, t) => s + (t.risk_score ?? 0), 0) / targets.length)
      : 0,
    criticals: 0,   // se poblarían con métricas tenant-level (endpoint futuro)
    highs:     0,
  });

  // Form estado
  let formKind  = $state<ScanTarget['kind']>('domain');
  let formName  = $state('');
  let formValue = $state('');
  let formBusy  = $state(false);

  // Timeline: métricas del primer target como proxy del tenant
  let tlCritical = $state<Metric[]>([]);
  let tlHigh     = $state<Metric[]>([]);
  let tlMedium   = $state<Metric[]>([]);
  let tlLow      = $state<Metric[]>([]);

  const columns = [
    { key: 'name',           label: 'Nombre',        sortable: true },
    { key: 'kind',           label: 'Tipo',          sortable: true, width: '90px' },
    { key: 'risk_score',     label: 'Riesgo',        width: '100px', align: 'center' as const },
    { key: 'last_scanned_at',label: 'Último scan',   sortable: true, width: '140px' },
  ];

  onMount(async () => {
    try {
      targets = await targetsApi.list();
      // Cargar métricas del primer target si existe
      if (targets.length > 0) {
        const id = targets[0].id;
        [tlCritical, tlHigh, tlMedium, tlLow] = await Promise.all([
          metricsApi.history(id, 'vuln_count_critical', 30),
          metricsApi.history(id, 'vuln_count_high',     30),
          metricsApi.history(id, 'vuln_count_medium',   30),
          metricsApi.history(id, 'vuln_count_low',      30),
        ]);
      }
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error cargando datos');
    } finally {
      loading = false;
    }
  });

  async function enqueueScan(id: string) {
    try {
      await targetsApi.enqueueScan(id);
      toasts.success('Scan encolado');
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error al encolar scan');
    }
  }

  async function createTarget() {
    formBusy = true;
    try {
      const t = await targetsApi.create({ kind: formKind, name: formName, value: formValue });
      targets = [...targets, t];
      showForm = false;
      formName = ''; formValue = '';
      toasts.success('Asset creado');
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error al crear asset');
    } finally {
      formBusy = false;
    }
  }

  function fmtDate(iso: string | null) {
    if (!iso) return '—';
    return new Date(iso).toLocaleDateString('es', { day: '2-digit', month: 'short', year: 'numeric' });
  }
</script>

<svelte:head><title>Dashboard — TriSecLabs</title></svelte:head>

<!-- KPI cards -->
<div class="grid grid-cols-2 gap-4 mb-6" style="grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));">
  {#each [
    { label: 'Assets',     value: kpis.total,    color: 'var(--accent)' },
    { label: 'Riesgo avg', value: kpis.avgRisk,  color: '#f97316' },
    { label: 'Críticos',   value: kpis.criticals, color: '#ef4444' },
    { label: 'Altos',      value: kpis.highs,     color: '#f97316' },
  ] as kpi}
    <div class="rounded-xl p-4"
         style="background: var(--bg-surface); border: 1px solid var(--border);">
      <p class="text-xs mb-1" style="color: var(--text-muted);">{kpi.label}</p>
      <p class="text-2xl font-bold" style="color: {kpi.color};">{kpi.value}</p>
    </div>
  {/each}
</div>

<!-- Gráfico timeline -->
{#if tlCritical.length > 0 || tlHigh.length > 0}
  <div class="rounded-xl p-4 mb-6"
       style="background: var(--bg-surface); border: 1px solid var(--border);">
    <p class="text-sm font-medium mb-3" style="color: var(--text-primary);">
      Tendencia de vulnerabilidades
    </p>
    <VulnTimeline critical={tlCritical} high={tlHigh} medium={tlMedium} low={tlLow} />
  </div>
{/if}

<!-- Tabla de targets -->
<div class="rounded-xl"
     style="background: var(--bg-surface); border: 1px solid var(--border);">
  <div class="flex items-center justify-between px-4 py-3"
       style="border-bottom: 1px solid var(--border);">
    <h2 class="text-sm font-semibold" style="color: var(--text-primary);">Assets</h2>
    <button
      onclick={() => showForm = !showForm}
      class="px-3 py-1.5 rounded text-xs font-medium"
      style="background: var(--accent); color: white;"
    >
      + Nuevo asset
    </button>
  </div>

  <!-- Form inline -->
  {#if showForm}
    <div class="px-4 py-3 flex gap-3 flex-wrap items-end"
         style="border-bottom: 1px solid var(--border); background: var(--bg-elevated);">
      <div>
        <label class="block text-xs mb-1" style="color: var(--text-muted);">Tipo</label>
        <select
          bind:value={formKind}
          class="px-2 py-1.5 rounded text-sm"
          style="background: var(--bg-surface); border: 1px solid var(--border); color: var(--text-primary);"
        >
          <option value="domain">Dominio</option>
          <option value="ip_range">IP / Rango</option>
          <option value="vendor">Proveedor</option>
          <option value="organization">Organización</option>
        </select>
      </div>
      <div>
        <label class="block text-xs mb-1" style="color: var(--text-muted);">Nombre</label>
        <input bind:value={formName} placeholder="Mi dominio" type="text"
               class="px-2 py-1.5 rounded text-sm"
               style="background: var(--bg-surface); border: 1px solid var(--border); color: var(--text-primary);" />
      </div>
      <div>
        <label class="block text-xs mb-1" style="color: var(--text-muted);">Valor</label>
        <input bind:value={formValue} placeholder="ejemplo.com" type="text"
               class="px-2 py-1.5 rounded text-sm"
               style="background: var(--bg-surface); border: 1px solid var(--border); color: var(--text-primary);" />
      </div>
      <button
        onclick={createTarget}
        disabled={formBusy || !formName || !formValue}
        class="px-3 py-1.5 rounded text-xs font-medium disabled:opacity-50"
        style="background: var(--accent); color: white;"
      >
        {formBusy ? 'Creando…' : 'Crear'}
      </button>
      <button onclick={() => showForm = false}
              class="px-3 py-1.5 rounded text-xs"
              style="background: var(--bg-elevated); color: var(--text-secondary);">
        Cancelar
      </button>
    </div>
  {/if}

  <DataTable rows={targets} {columns} {loading} emptyLabel="Sin assets. Crea el primero.">
    {#snippet cell({ row, col })}
      {#if col.key === 'risk_score'}
        {#if row.risk_level}
          <RiskBadge value={row.risk_level} />
        {:else}
          <span style="color: var(--text-muted);">—</span>
        {/if}
      {:else if col.key === 'last_scanned_at'}
        <span style="color: var(--text-secondary); font-size: 12px;">
          {fmtDate(row.last_scanned_at)}
        </span>
      {:else if col.key === 'name'}
        <a href="/assets/{row.id}"
           class="font-medium hover:underline"
           style="color: var(--text-primary);">
          {row.name}
        </a>
      {:else}
        <span style="color: var(--text-secondary);">{(row as unknown as Record<string, unknown>)[col.key] ?? '—'}</span>
      {/if}
    {/snippet}

    {#snippet rowActions({ row })}
      <button
        onclick={() => enqueueScan(row.id)}
        class="px-2 py-1 rounded text-xs"
        style="background: var(--bg-elevated); color: var(--text-secondary);"
        title="Lanzar scan"
      >
        Scan
      </button>
    {/snippet}
  </DataTable>
</div>
