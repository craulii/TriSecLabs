<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { targetsApi }  from '$lib/api/targets';
  import { toasts }      from '$lib/stores/ui';
  import { ApiError }    from '$lib/api/client';
  import RiskBadge       from '$lib/components/common/RiskBadge.svelte';
  import ScanLiveDrawer  from '$lib/components/scan/ScanLiveDrawer.svelte';
  import Play            from '$lib/icons/Play.svelte';
  import Loader          from '$lib/icons/Loader.svelte';
  import Bot             from '$lib/icons/Bot.svelte';
  import Pencil          from '$lib/icons/Pencil.svelte';
  import Trash           from '$lib/icons/Trash.svelte';
  import Search          from '$lib/icons/Search.svelte';
  import ChevronUp       from '$lib/icons/ChevronUp.svelte';
  import ChevronDown     from '$lib/icons/ChevronDown.svelte';
  import ChevronUpDown   from '$lib/icons/ChevronUpDown.svelte';
  import type { ScanTarget, TargetKind } from '$lib/types/models';

  // ─── State ───────────────────────────────────────────────────────────────────

  let targets    = $state<ScanTarget[]>([]);
  let loading    = $state(true);
  let search     = $state('');
  let sortKey    = $state<keyof ScanTarget | ''>('created_at');
  let sortDir    = $state<'asc' | 'desc'>('desc');

  // Scan en vivo: drawer abierto cuando hay un job_id activo
  let activeJobId      = $state<string | null>(null);
  let activeTargetName = $state<string>('');

  // Edit modal
  let editTarget = $state<ScanTarget | null>(null);
  let editName   = $state('');
  let editValue  = $state('');
  let editSaving = $state(false);

  // Delete confirmation
  let deleteTarget  = $state<ScanTarget | null>(null);
  let deleteLoading = $state(false);

  // ─── Reglas de tipo de asset ─────────────────────────────────────────────────

  function isScannable(kind: TargetKind): boolean {
    return kind === 'domain' || kind === 'ip_range';
  }

  // ─── Data ────────────────────────────────────────────────────────────────────

  async function loadTargets() {
    try {
      targets = await targetsApi.list();
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error cargando assets');
    } finally {
      loading = false;
    }
  }

  // ─── Filtered + sorted ───────────────────────────────────────────────────────

  const filtered = $derived.by(() => {
    let list = search.trim()
      ? targets.filter(t =>
          t.name.toLowerCase().includes(search.toLowerCase()) ||
          t.value.toLowerCase().includes(search.toLowerCase())
        )
      : targets;

    if (!sortKey) return list;
    return [...list].sort((a, b) => {
      const av = a[sortKey as keyof ScanTarget] ?? '';
      const bv = b[sortKey as keyof ScanTarget] ?? '';
      const cmp = String(av).localeCompare(String(bv), undefined, { numeric: true });
      return sortDir === 'asc' ? cmp : -cmp;
    });
  });

  function setSort(key: keyof ScanTarget) {
    if (sortKey === key) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    } else {
      sortKey = key;
      sortDir = key === 'last_scanned_at' || key === 'created_at' ? 'desc' : 'asc';
    }
  }

  // ─── Scan ─────────────────────────────────────────────────────────────────────

  async function enqueueScan(target: ScanTarget) {
    if (!isScannable(target.kind)) {
      toasts.warning('Este tipo de asset no es escaneable directamente. Requiere OSINT manual.');
      return;
    }
    try {
      const { job_id } = await targetsApi.enqueueScan(target.id);
      activeJobId      = job_id;
      activeTargetName = target.name;
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error al encolar scan');
    }
  }

  function closeDrawer() {
    activeJobId      = null;
    activeTargetName = '';
    loadTargets();
  }

  async function enqueueLlm(id: string) {
    try {
      await targetsApi.enqueueLlmReport(id);
      toasts.warning('Informe LLM encolado. Requiere servidor LLM activo para procesarse.');
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error al encolar informe');
    }
  }

  // ─── Edit ─────────────────────────────────────────────────────────────────────

  function openEdit(t: ScanTarget) {
    editTarget = t;
    editName   = t.name;
    editValue  = t.value;
  }

  function closeEdit() {
    editTarget = null;
    editName   = '';
    editValue  = '';
  }

  async function saveEdit() {
    if (!editTarget) return;
    editSaving = true;
    try {
      const updated = await targetsApi.update(editTarget.id, { name: editName, value: editValue });
      targets = targets.map(t => t.id === updated.id ? updated : t);
      closeEdit();
      toasts.success('Asset actualizado');
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error al actualizar');
    } finally {
      editSaving = false;
    }
  }

  // ─── Delete ───────────────────────────────────────────────────────────────────

  function openDelete(t: ScanTarget) {
    deleteTarget = t;
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    deleteLoading = true;
    try {
      await targetsApi.delete(deleteTarget.id);
      targets = targets.filter(t => t.id !== deleteTarget!.id);
      deleteTarget = null;
      toasts.success('Asset eliminado');
    } catch (err) {
      toasts.error(err instanceof ApiError ? err.message : 'Error al eliminar');
    } finally {
      deleteLoading = false;
    }
  }

  // ─── Helpers ─────────────────────────────────────────────────────────────────

  function fmtDate(iso: string | null) {
    if (!iso) return '—';
    return new Date(iso).toLocaleDateString('es', { day: '2-digit', month: 'short', year: 'numeric' });
  }

  onMount(loadTargets);
  onDestroy(() => { activeJobId = null; });
</script>

<svelte:head><title>Assets — TriSecLabs</title></svelte:head>

<!-- ─── Edit modal ─────────────────────────────────────────────────────────── -->
{#if editTarget}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    style="background: rgba(0,0,0,0.5);"
    role="dialog" aria-modal="true"
  >
    <div class="w-full max-w-md"
         style="background: var(--bg-surface); border: 1px solid var(--border); padding: var(--space-6); border-radius: var(--radius-xl); box-shadow: var(--shadow-lg);">
      <h2 style="font-size: var(--font-size-md); font-weight: var(--font-weight-semibold); color: var(--text-primary); margin-bottom: var(--space-4);">
        Editar asset
      </h2>

      <div style="display: flex; flex-direction: column; gap: var(--space-3);">
        <div>
          <label for="edit-name" style="display: block; font-size: var(--font-size-sm); color: var(--text-secondary); margin-bottom: var(--space-1);">Nombre</label>
          <input
            id="edit-name"
            bind:value={editName}
            class="w-full outline-none"
            style="padding: var(--space-2) var(--space-3); background: var(--bg-elevated); border: 1px solid var(--border); color: var(--text-primary); border-radius: var(--radius-md); font-size: var(--font-size-sm);"
          />
        </div>
        <div>
          <label for="edit-value" style="display: block; font-size: var(--font-size-sm); color: var(--text-secondary); margin-bottom: var(--space-1);">Valor ({editTarget.kind})</label>
          <input
            id="edit-value"
            bind:value={editValue}
            class="w-full outline-none"
            style="padding: var(--space-2) var(--space-3); background: var(--bg-elevated); border: 1px solid var(--border); color: var(--text-primary); border-radius: var(--radius-md); font-size: var(--font-size-sm);"
          />
        </div>
      </div>

      <div style="display: flex; justify-content: flex-end; gap: var(--space-2); margin-top: var(--space-5);">
        <button onclick={closeEdit}
                style="padding: var(--space-2) var(--space-4); background: var(--bg-elevated); color: var(--text-secondary); border: none; border-radius: var(--radius-md); font-size: var(--font-size-sm); cursor: pointer;">
          Cancelar
        </button>
        <button onclick={saveEdit}
                disabled={editSaving || !editName.trim() || !editValue.trim()}
                style="padding: var(--space-2) var(--space-4); background: var(--accent); color: #fff; border: none; border-radius: var(--radius-md); font-size: var(--font-size-sm); font-weight: var(--font-weight-medium); cursor: pointer; opacity: {editSaving ? 0.6 : 1};">
          {editSaving ? 'Guardando...' : 'Guardar'}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- ─── Delete modal ───────────────────────────────────────────────────────── -->
{#if deleteTarget}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    style="background: rgba(0,0,0,0.5);"
    role="dialog" aria-modal="true"
  >
    <div class="w-full max-w-sm"
         style="background: var(--bg-surface); border: 1px solid var(--border); padding: var(--space-6); border-radius: var(--radius-xl); box-shadow: var(--shadow-lg);">
      <h2 style="font-size: var(--font-size-md); font-weight: var(--font-weight-semibold); color: var(--text-primary); margin-bottom: var(--space-2);">
        Eliminar asset
      </h2>
      <p style="font-size: var(--font-size-sm); color: var(--text-secondary); margin-bottom: var(--space-5);">
        ¿Eliminar <strong style="color: var(--text-primary);">{deleteTarget.name}</strong>?
        Se borrarán también todos sus puertos y vulnerabilidades. Esta acción no se puede deshacer.
      </p>
      <div style="display: flex; justify-content: flex-end; gap: var(--space-2);">
        <button onclick={() => deleteTarget = null}
                style="padding: var(--space-2) var(--space-4); background: var(--bg-elevated); color: var(--text-secondary); border: none; border-radius: var(--radius-md); font-size: var(--font-size-sm); cursor: pointer;">
          Cancelar
        </button>
        <button onclick={confirmDelete}
                disabled={deleteLoading}
                style="padding: var(--space-2) var(--space-4); background: var(--sev-critical); color: #fff; border: none; border-radius: var(--radius-md); font-size: var(--font-size-sm); font-weight: var(--font-weight-medium); cursor: pointer; opacity: {deleteLoading ? 0.6 : 1};">
          {deleteLoading ? 'Eliminando...' : 'Eliminar'}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- ─── Scan live drawer ─────────────────────────────────────────────────── -->
{#if activeJobId}
  <ScanLiveDrawer
    jobId={activeJobId}
    targetName={activeTargetName}
    onClose={closeDrawer}
  />
{/if}

<!-- ─── Main ───────────────────────────────────────────────────────────────── -->
<div style="max-width: var(--max-content-width); margin: 0 auto;">
  <header style="margin-bottom: var(--space-6);">
    <h1 style="font-size: var(--font-size-xl); font-weight: var(--font-weight-semibold); color: var(--text-primary);">
      Assets
    </h1>
    <p style="font-size: var(--font-size-sm); color: var(--text-muted); margin-top: var(--space-1);">
      Gestión de activos monitoreados — escaneo, puertos y vulnerabilidades.
    </p>
  </header>

  <div
    style="background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-xl); overflow: hidden; box-shadow: var(--shadow-sm);"
  >
    <!-- Header del card -->
    <div
      style="padding: var(--space-4) var(--space-5); display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); flex-wrap: wrap; border-bottom: 1px solid var(--border);"
    >
      <div>
        <h2 style="font-size: var(--font-size-md); font-weight: var(--font-weight-semibold); color: var(--text-primary);">
          {filtered.length} de {targets.length} activo{targets.length !== 1 ? 's' : ''}
        </h2>
      </div>

      <!-- Search -->
      <div style="position: relative; flex: 1 1 auto; min-width: 220px; max-width: 320px;">
        <span style="position: absolute; left: var(--space-3); top: 50%; transform: translateY(-50%); color: var(--text-muted); pointer-events: none;">
          <Search size={16} />
        </span>
        <input
          bind:value={search}
          placeholder="Buscar por nombre o valor…"
          class="w-full outline-none"
          style="padding: var(--space-2) var(--space-3) var(--space-2) var(--space-10); background: var(--bg-elevated); border: 1px solid var(--border); color: var(--text-primary); border-radius: var(--radius-md); font-size: var(--font-size-sm);"
        />
      </div>
    </div>

    <!-- Sort bar -->
    <div
      style="padding: var(--space-3) var(--space-5); display: flex; align-items: center; gap: var(--space-2); flex-wrap: wrap; border-bottom: 1px solid var(--border); color: var(--text-muted); font-size: var(--font-size-sm);"
    >
      <span style="margin-right: var(--space-2);">Ordenar:</span>
      {#each [
        { key: 'name' as const,            label: 'Nombre' },
        { key: 'kind' as const,            label: 'Tipo' },
        { key: 'risk_score' as const,      label: 'Riesgo' },
        { key: 'last_scanned_at' as const, label: 'Último scan' },
      ] as col}
        <button
          onclick={() => setSort(col.key)}
          class="sort-btn"
          style="
            display: inline-flex; align-items: center; gap: var(--space-1);
            padding: var(--space-1) var(--space-3); border-radius: var(--radius-md);
            background: {sortKey === col.key ? 'var(--accent)' : 'var(--bg-elevated)'};
            color:      {sortKey === col.key ? '#fff' : 'var(--text-secondary)'};
            border: none; cursor: pointer; font-size: var(--font-size-xs);
          "
        >
          {col.label}
          {#if sortKey === col.key}
            {#if sortDir === 'asc'}
              <ChevronUp size={12} />
            {:else}
              <ChevronDown size={12} />
            {/if}
          {:else}
            <ChevronUpDown size={12} />
          {/if}
        </button>
      {/each}
    </div>

    <!-- Table -->
    <div style="width: 100%; overflow-x: auto;">
      <table style="width: 100%; border-collapse: collapse; font-size: var(--font-size-sm);">
        <thead>
          <tr style="border-bottom: 1px solid var(--border);">
            <th style="padding: var(--space-3) var(--space-5); text-align: left; font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.05em;">Nombre</th>
            <th style="padding: var(--space-3) var(--space-5); text-align: left; font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.05em; width: 120px;">Tipo</th>
            <th style="padding: var(--space-3) var(--space-5); text-align: left; font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.05em;">Valor</th>
            <th style="padding: var(--space-3) var(--space-5); text-align: center; font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.05em; width: 120px;">Riesgo</th>
            <th style="padding: var(--space-3) var(--space-5); text-align: left; font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.05em; width: 160px;">Último scan</th>
            <th style="padding: var(--space-3) var(--space-5); width: 220px;"></th>
          </tr>
        </thead>
        <tbody>
          {#if loading}
            {#each Array(4) as _}
              <tr style="border-bottom: 1px solid var(--border);">
                {#each Array(6) as _}
                  <td style="padding: var(--space-4) var(--space-5);">
                    <div style="height: 12px; background: var(--bg-elevated); border-radius: var(--radius-sm); width: 75%;" class="animate-pulse"></div>
                  </td>
                {/each}
              </tr>
            {/each}
          {:else if filtered.length === 0}
            <tr>
              <td colspan="6" style="padding: var(--space-12) var(--space-5); text-align: center; font-size: var(--font-size-sm); color: var(--text-muted);">
                {search ? 'Sin resultados para esa búsqueda' : 'Sin assets configurados'}
              </td>
            </tr>
          {:else}
            {#each filtered as row (row.id)}
              {@const scannable = isScannable(row.kind)}
              <tr
                style="border-bottom: 1px solid var(--border); transition: background 150ms;"
                onmouseenter={e => (e.currentTarget as HTMLElement).style.background = 'var(--bg-elevated)'}
                onmouseleave={e => (e.currentTarget as HTMLElement).style.background = 'transparent'}
              >
                <td style="padding: var(--space-4) var(--space-5);">
                  <a href="/assets/{row.id}" style="font-weight: var(--font-weight-medium); color: var(--text-primary); text-decoration: none;">
                    {row.name}
                  </a>
                </td>

                <td style="padding: var(--space-4) var(--space-5);">
                  <span style="display: inline-block; padding: 2px var(--space-2); border-radius: var(--radius-sm); background: var(--bg-elevated); color: var(--text-secondary); border: 1px solid var(--border); font-size: var(--font-size-xs); text-transform: uppercase; letter-spacing: 0.03em;">
                    {row.kind}
                  </span>
                </td>

                <td style="padding: var(--space-4) var(--space-5);">
                  <span style="font-family: ui-monospace, monospace; color: var(--text-secondary); font-size: var(--font-size-sm);">
                    {row.value}
                  </span>
                </td>

                <td style="padding: var(--space-4) var(--space-5); text-align: center;">
                  {#if row.risk_level}
                    <RiskBadge value={row.risk_level} />
                  {:else}
                    <span style="color: var(--text-muted);">—</span>
                  {/if}
                </td>

                <td style="padding: var(--space-4) var(--space-5); color: var(--text-secondary); font-size: var(--font-size-sm);">
                  {fmtDate(row.last_scanned_at)}
                </td>

                <td style="padding: var(--space-4) var(--space-5);">
                  <div style="display: flex; align-items: center; gap: var(--space-1); justify-content: flex-end;">
                    <!-- Scan -->
                    <button
                      onclick={() => enqueueScan(row)}
                      disabled={!scannable}
                      title={scannable ? 'Lanzar scan' : 'No escaneable directamente — requiere OSINT manual'}
                      class="action-btn primary-action"
                    >
                      <Play size={14} />
                      <span>Scan</span>
                    </button>

                    <!-- LLM -->
                    <button
                      onclick={() => enqueueLlm(row.id)}
                      title="Generar informe LLM"
                      class="action-btn"
                    >
                      <Bot size={14} />
                    </button>

                    <!-- Edit -->
                    <button
                      onclick={() => openEdit(row)}
                      title="Editar"
                      class="action-btn"
                    >
                      <Pencil size={14} />
                    </button>

                    <!-- Delete -->
                    <button
                      onclick={() => openDelete(row)}
                      title="Eliminar"
                      class="action-btn danger-action"
                    >
                      <Trash size={14} />
                    </button>
                  </div>
                </td>
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
    </div>
  </div>
</div>

<style>
  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: var(--font-size-xs);
    cursor: pointer;
    transition: background 150ms, color 150ms, border-color 150ms;
  }
  .action-btn:hover:not(:disabled) {
    background: var(--bg-surface);
    color: var(--text-primary);
    border-color: var(--text-muted);
  }
  .action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .primary-action:hover:not(:disabled) {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  .danger-action:hover:not(:disabled) {
    background: var(--sev-critical);
    border-color: var(--sev-critical);
    color: #fff;
  }
  .animate-pulse {
    animation: pulse 1.6s cubic-bezier(.4,0,.6,1) infinite;
  }
  @keyframes pulse {
    50% { opacity: 0.5; }
  }
</style>
