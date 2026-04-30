<script lang="ts">
  import { onMount }          from 'svelte';
  import { page }             from '$app/stores';
  import { targetsApi }       from '$lib/api/targets';
  import { vulnsApi }         from '$lib/api/vulnerabilities';
  import { metricsApi }       from '$lib/api/metrics';
  import { toasts }           from '$lib/stores/ui';
  import { ApiError }         from '$lib/api/client';
  import RiskGauge            from '$lib/components/charts/RiskGauge.svelte';
  import VulnTimeline         from '$lib/components/charts/VulnTimeline.svelte';
  import DataTable            from '$lib/components/common/DataTable.svelte';
  import RiskBadge            from '$lib/components/common/RiskBadge.svelte';
  import type { ScanTarget, ExposedPort, Vulnerability, Metric, VulnStatus } from '$lib/types/models';

  const id = $derived($page.params.id as string);

  let target  = $state<ScanTarget | null>(null);
  let ports   = $state<ExposedPort[]>([]);
  let vulns   = $state<Vulnerability[]>([]);
  let kpis    = $state({ riskScore: 0, critical: 0, high: 0, exposedPorts: 0 });
  let loading = $state(true);

  let tlCritical = $state<Metric[]>([]);
  let tlHigh     = $state<Metric[]>([]);
  let tlMedium   = $state<Metric[]>([]);
  let tlLow      = $state<Metric[]>([]);

  const portColumns = [
    { key: 'port',     label: 'Puerto',   sortable: true, width: '80px' },
    { key: 'protocol', label: 'Protocolo',width: '90px' },
    { key: 'service',  label: 'Servicio', sortable: true },
    { key: 'product',  label: 'Producto', sortable: true },
    { key: 'version',  label: 'Versión' },
    { key: 'state',    label: 'Estado',   width: '90px' },
  ];

  const vulnColumns = [
    { key: 'severity', label: 'Severidad', width: '110px' },
    { key: 'title',    label: 'Título',    sortable: true },
    { key: 'cve_id',   label: 'CVE',       width: '130px' },
    { key: 'status',   label: 'Estado',    width: '130px' },
    { key: 'source',   label: 'Fuente',    width: '110px' },
  ];

  onMount(async () => {
    try {
      [target, ports, kpis, vulns] = await Promise.all([
        targetsApi.get(id),
        targetsApi.listPorts(id),
        metricsApi.latest(id),
        vulnsApi.listForTarget(id),
      ]);

      [tlCritical, tlHigh, tlMedium, tlLow] = await Promise.all([
        metricsApi.history(id, 'vuln_count_critical', 30),
        metricsApi.history(id, 'vuln_count_high',     30),
        metricsApi.history(id, 'vuln_count_medium',   30),
        metricsApi.history(id, 'vuln_count_low',      30),
      ]);
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error cargando datos del asset');
    } finally {
      loading = false;
    }
  });

  async function changeStatus(vulnId: string, status: VulnStatus) {
    try {
      await vulnsApi.updateStatus(vulnId, status);
      vulns = vulns.map(v => v.id === vulnId ? { ...v, status } : v);
      toasts.success('Estado actualizado');
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error al actualizar');
    }
  }

  function fmtDate(iso: string | null) {
    if (!iso) return '—';
    return new Date(iso).toLocaleDateString('es', { day: '2-digit', month: 'short', year: 'numeric' });
  }
</script>

<svelte:head>
  <title>{target?.name ?? 'Asset'} — TriSecLabs</title>
</svelte:head>

<!-- Breadcrumb -->
<div class="flex items-center gap-2 text-sm mb-4" style="color: var(--text-muted);">
  <a href="/assets" style="color: var(--accent);">Assets</a>
  <span>›</span>
  <span style="color: var(--text-primary);">{target?.name ?? '…'}</span>
</div>

<!-- Header -->
{#if target}
  <div class="flex items-center gap-3 mb-6">
    <div>
      <h1 class="text-xl font-bold" style="color: var(--text-primary);">{target.name}</h1>
      <p class="text-sm mt-0.5" style="color: var(--text-muted);">
        {target.kind} · {target.value}
      </p>
    </div>
  </div>
{/if}

<!-- Top grid: gauge + KPIs + timeline -->
<div class="grid gap-4 mb-6" style="grid-template-columns: 220px 1fr;">
  <!-- Gauge + KPIs -->
  <div class="rounded-xl p-4"
       style="background: var(--bg-surface); border: 1px solid var(--border);">
    <p class="text-xs font-medium mb-1" style="color: var(--text-muted);">Risk Score</p>
    <RiskGauge score={kpis.riskScore} />
    <div class="grid grid-cols-2 gap-2 mt-2">
      <div class="text-center">
        <p class="text-lg font-bold" style="color: #ef4444;">{kpis.critical}</p>
        <p class="text-xs" style="color: var(--text-muted);">Críticos</p>
      </div>
      <div class="text-center">
        <p class="text-lg font-bold" style="color: #f97316;">{kpis.high}</p>
        <p class="text-xs" style="color: var(--text-muted);">Altos</p>
      </div>
      <div class="text-center col-span-2">
        <p class="text-lg font-bold" style="color: var(--accent);">{kpis.exposedPorts}</p>
        <p class="text-xs" style="color: var(--text-muted);">Puertos expuestos</p>
      </div>
    </div>
  </div>

  <!-- Timeline -->
  <div class="rounded-xl p-4"
       style="background: var(--bg-surface); border: 1px solid var(--border);">
    <p class="text-xs font-medium mb-2" style="color: var(--text-muted);">
      Evolución (30 días)
    </p>
    <VulnTimeline critical={tlCritical} high={tlHigh} medium={tlMedium} low={tlLow} />
  </div>
</div>

<!-- Tabla de puertos -->
<div class="rounded-xl mb-4"
     style="background: var(--bg-surface); border: 1px solid var(--border);">
  <div class="px-4 py-3" style="border-bottom: 1px solid var(--border);">
    <h2 class="text-sm font-semibold" style="color: var(--text-primary);">
      Puertos expuestos ({ports.length})
    </h2>
  </div>
  <DataTable rows={ports} columns={portColumns} {loading} emptyLabel="Sin puertos detectados">
    {#snippet cell({ row, col })}
      {#if col.key === 'state'}
        <span class="text-xs px-1.5 py-0.5 rounded"
              style="
                background: {row.state === 'open' ? 'color-mix(in srgb, #22c55e 15%, transparent)' : 'var(--bg-elevated)'};
                color: {row.state === 'open' ? '#22c55e' : 'var(--text-muted)'};
              ">
          {row.state}
        </span>
      {:else}
        <span style="color: var(--text-secondary); font-size: 13px;">{(row as unknown as Record<string, unknown>)[col.key] ?? '—'}</span>
      {/if}
    {/snippet}
  </DataTable>
</div>

<!-- Tabla de vulnerabilidades -->
<div class="rounded-xl"
     style="background: var(--bg-surface); border: 1px solid var(--border);">
  <div class="px-4 py-3" style="border-bottom: 1px solid var(--border);">
    <h2 class="text-sm font-semibold" style="color: var(--text-primary);">
      Vulnerabilidades ({vulns.length})
    </h2>
  </div>
  <DataTable rows={vulns} columns={vulnColumns} {loading} emptyLabel="Sin vulnerabilidades detectadas">
    {#snippet cell({ row, col })}
      {#if col.key === 'severity'}
        <RiskBadge value={row.severity} />
      {:else if col.key === 'status'}
        <RiskBadge value={row.status} variant="status" />
      {:else if col.key === 'title'}
        <span class="text-sm" style="color: var(--text-primary);">{row.title}</span>
      {:else}
        <span style="color: var(--text-secondary); font-size: 13px;">{(row as unknown as Record<string, unknown>)[col.key] ?? '—'}</span>
      {/if}
    {/snippet}

    {#snippet rowActions({ row })}
      <select
        value={row.status}
        onchange={(e) => changeStatus(row.id, (e.currentTarget as HTMLSelectElement).value as VulnStatus)}
        class="px-1.5 py-1 rounded text-xs"
        style="background: var(--bg-elevated); border: 1px solid var(--border); color: var(--text-secondary);"
      >
        <option value="open">Open</option>
        <option value="in_progress">In Progress</option>
        <option value="mitigated">Mitigated</option>
        <option value="resolved">Resolved</option>
        <option value="accepted">Accepted</option>
        <option value="false_positive">False Positive</option>
      </select>
    {/snippet}
  </DataTable>
</div>
