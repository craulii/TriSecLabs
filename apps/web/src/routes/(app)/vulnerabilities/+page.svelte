<script lang="ts">
  import { onMount }     from 'svelte';
  import { vulnsApi }    from '$lib/api/vulnerabilities';
  import { toasts }      from '$lib/stores/ui';
  import { ApiError }    from '$lib/api/client';
  import DataTable       from '$lib/components/common/DataTable.svelte';
  import Pagination      from '$lib/components/common/Pagination.svelte';
  import RiskBadge       from '$lib/components/common/RiskBadge.svelte';
  import type { Vulnerability, VulnStatus, RiskLevel } from '$lib/types/models';

  const LIMIT = 25;

  let vulns    = $state<Vulnerability[]>([]);
  let total    = $state(0);
  let page     = $state(1);
  let loading  = $state(true);

  let filterSeverity = $state('');
  let filterStatus   = $state('');
  let filterSearch   = $state('');

  // Filtrado client-side sobre los resultados cargados
  let filtered = $derived(
    vulns.filter(v => {
      if (filterSeverity && v.severity !== filterSeverity) return false;
      if (filterStatus   && v.status   !== filterStatus)   return false;
      if (filterSearch) {
        const q = filterSearch.toLowerCase();
        return (
          v.title.toLowerCase().includes(q) ||
          (v.cve_id ?? '').toLowerCase().includes(q) ||
          (v.cwe_id ?? '').toLowerCase().includes(q)
        );
      }
      return true;
    })
  );

  const columns = [
    { key: 'severity',     label: 'Severidad', width: '110px' },
    { key: 'title',        label: 'Título',    sortable: true },
    { key: 'cve_id',       label: 'CVE',       width: '130px' },
    { key: 'status',       label: 'Estado',    width: '130px' },
    { key: 'source',       label: 'Fuente',    width: '110px' },
    { key: 'first_seen_at',label: 'Detectado', sortable: true, width: '120px' },
  ];

  async function load() {
    loading = true;
    try {
      vulns = await vulnsApi.list({
        page,
        limit: LIMIT,
        severity: filterSeverity || undefined,
        status:   filterStatus   || undefined,
      });
      total = vulns.length; // El backend debería devolver PaginatedResponse; esto es fallback
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error cargando vulnerabilidades');
    } finally {
      loading = false;
    }
  }

  onMount(load);

  function onPageChange(p: number) {
    page = p;
    load();
  }

  async function changeStatus(id: string, status: VulnStatus) {
    try {
      await vulnsApi.updateStatus(id, status);
      vulns = vulns.map(v => v.id === id ? { ...v, status } : v);
      toasts.success('Estado actualizado');
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error al actualizar');
    }
  }

  function fmtDate(iso: string) {
    return new Date(iso).toLocaleDateString('es', { day: '2-digit', month: 'short', year: 'numeric' });
  }
</script>

<svelte:head><title>Vulnerabilidades — TriSecLabs</title></svelte:head>

<div class="rounded-xl overflow-hidden"
     style="background: var(--bg-surface); border: 1px solid var(--border);">

  <!-- Header + filtros -->
  <div class="px-4 py-3" style="border-bottom: 1px solid var(--border);">
    <div class="flex items-center justify-between mb-3">
      <h1 class="text-sm font-semibold" style="color: var(--text-primary);">Vulnerabilidades</h1>
      <span class="text-xs" style="color: var(--text-muted);">{filtered.length} resultado{filtered.length !== 1 ? 's' : ''}</span>
    </div>

    <div class="flex flex-wrap gap-2">
      <!-- Búsqueda -->
      <input
        type="search"
        bind:value={filterSearch}
        placeholder="Buscar título, CVE…"
        class="px-2.5 py-1.5 rounded text-sm flex-1"
        style="min-width: 180px; background: var(--bg-elevated); border: 1px solid var(--border); color: var(--text-primary);"
      />

      <!-- Filtro severidad -->
      <select
        bind:value={filterSeverity}
        class="px-2.5 py-1.5 rounded text-sm"
        style="background: var(--bg-elevated); border: 1px solid var(--border); color: var(--text-secondary);"
      >
        <option value="">Todas las severidades</option>
        <option value="critical">Critical</option>
        <option value="high">High</option>
        <option value="medium">Medium</option>
        <option value="low">Low</option>
        <option value="info">Info</option>
      </select>

      <!-- Filtro estado -->
      <select
        bind:value={filterStatus}
        class="px-2.5 py-1.5 rounded text-sm"
        style="background: var(--bg-elevated); border: 1px solid var(--border); color: var(--text-secondary);"
      >
        <option value="">Todos los estados</option>
        <option value="open">Open</option>
        <option value="in_progress">In Progress</option>
        <option value="mitigated">Mitigated</option>
        <option value="resolved">Resolved</option>
        <option value="accepted">Accepted</option>
        <option value="false_positive">False Positive</option>
      </select>
    </div>
  </div>

  <DataTable rows={filtered} {columns} {loading} emptyLabel="Sin vulnerabilidades que coincidan">
    {#snippet cell({ row, col })}
      {#if col.key === 'severity'}
        <RiskBadge value={row.severity} />
      {:else if col.key === 'status'}
        <RiskBadge value={row.status} variant="status" />
      {:else if col.key === 'title'}
        <span class="text-sm" style="color: var(--text-primary);">{row.title}</span>
      {:else if col.key === 'first_seen_at'}
        <span style="color: var(--text-muted); font-size: 12px;">{fmtDate(row.first_seen_at)}</span>
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

  <!-- Paginación (solo visible si hay más de una página del servidor) -->
  {#if total > LIMIT}
    <div class="px-4" style="border-top: 1px solid var(--border);">
      <Pagination {page} limit={LIMIT} {total} onchange={onPageChange} />
    </div>
  {/if}
</div>
