<script lang="ts">
  import { jobsApi }     from '$lib/api/jobs';
  import { onDestroy }   from 'svelte';
  import type { JobProgressEvent, ScanStage } from '$lib/types/models';
  import ProgressBar     from '$lib/components/common/ProgressBar.svelte';
  import X               from '$lib/icons/X.svelte';
  import Activity        from '$lib/icons/Activity.svelte';
  import Check           from '$lib/icons/Check.svelte';
  import AlertCircle     from '$lib/icons/AlertCircle.svelte';
  import Loader          from '$lib/icons/Loader.svelte';

  let { jobId, targetName, onClose }: {
    jobId: string;
    targetName: string;
    onClose: () => void;
  } = $props();

  let event = $state<JobProgressEvent | null>(null);
  let connectionError = $state(false);
  let es: EventSource | null = null;

  const STAGES: { key: ScanStage; label: string }[] = [
    { key: 'validating',        label: 'Validación' },
    { key: 'starting',          label: 'Iniciando nmap' },
    { key: 'host_discovery',    label: 'Descubrimiento de host' },
    { key: 'port_scan',         label: 'Escaneo de puertos' },
    { key: 'service_detection', label: 'Detección de servicios' },
    { key: 'vulners',           label: 'Análisis de vulnerabilidades' },
    { key: 'persisting',        label: 'Guardando resultados' },
  ];

  const stageIndex = $derived.by(() => {
    if (!event?.stage) return -1;
    if (event.status === 'done')   return STAGES.length;
    if (event.status === 'failed') return -1;
    return STAGES.findIndex(s => s.key === event!.stage);
  });

  const isTerminal = $derived(event?.status === 'done' || event?.status === 'failed');

  function stageStatus(idx: number): 'done' | 'active' | 'pending' {
    if (event?.status === 'failed') return idx <= stageIndex ? 'done' : 'pending';
    if (event?.status === 'done')   return 'done';
    if (idx <  stageIndex) return 'done';
    if (idx === stageIndex) return 'active';
    return 'pending';
  }

  $effect(() => {
    if (!jobId) return;
    connectionError = false;
    event = null;

    es = jobsApi.stream(
      jobId,
      (ev) => { event = ev; },
      () => { connectionError = true; },
    );

    return () => {
      es?.close();
      es = null;
    };
  });

  onDestroy(() => { es?.close(); });

  const stageLabel = $derived.by(() => {
    if (!event) return 'Conectando…';
    if (event.status === 'failed') return 'Falló';
    if (event.status === 'done')   return 'Completado';
    const cur = STAGES.find(s => s.key === event!.stage);
    return cur?.label ?? 'En curso';
  });

  const barColor = $derived(
    event?.status === 'failed' ? 'var(--sev-critical)' :
    event?.status === 'done'   ? 'var(--sev-low)' :
    'var(--accent)'
  );
</script>

<!-- Backdrop semi-transparente que cierra al hacer click fuera -->
<div
  class="drawer-backdrop"
  onclick={onClose}
  onkeydown={(e) => { if (e.key === 'Escape') onClose(); }}
  role="presentation"
></div>

<div
  class="drawer"
  role="dialog"
  aria-modal="true"
  aria-labelledby="drawer-title"
  style="background: var(--bg-surface); border-left: 1px solid var(--border); box-shadow: var(--shadow-lg);"
>
  <!-- Header -->
  <header class="drawer-header" style="border-bottom: 1px solid var(--border);">
    <div class="header-title">
      <div class="status-icon" style="color: {barColor};">
        {#if event?.status === 'done'}
          <Check size={20} />
        {:else if event?.status === 'failed'}
          <AlertCircle size={20} />
        {:else}
          <Activity size={20} />
        {/if}
      </div>
      <div>
        <h2 id="drawer-title" style="font-size: var(--font-size-md); font-weight: var(--font-weight-semibold); color: var(--text-primary);">
          {targetName}
        </h2>
        <p style="font-size: var(--font-size-xs); color: var(--text-muted); margin-top: 2px;">
          {stageLabel}
        </p>
      </div>
    </div>
    <button
      class="close-btn"
      onclick={onClose}
      aria-label="Cerrar"
      style="color: var(--text-secondary); background: transparent; border: none; cursor: pointer; padding: var(--space-2); border-radius: var(--radius-md);"
    >
      <X size={18} />
    </button>
  </header>

  <!-- Body scrollable -->
  <div class="drawer-body">
    <!-- Progress bar -->
    <section style="padding: var(--space-5);">
      <ProgressBar
        value={event?.progress ?? 0}
        label={stageLabel}
        indeterminate={!event || (event.progress === null && !isTerminal)}
        color={barColor}
        size="md"
      />
    </section>

    <!-- Error message -->
    {#if event?.status === 'failed' && event.error}
      <section
        style="margin: 0 var(--space-5) var(--space-5); padding: var(--space-4); border-radius: var(--radius-lg);
               background: rgba(239, 68, 68, 0.08); border: 1px solid rgba(239, 68, 68, 0.3);"
      >
        <div style="display: flex; gap: var(--space-3); align-items: flex-start;">
          <span style="color: var(--sev-critical); flex-shrink: 0; margin-top: 2px;">
            <AlertCircle size={16} />
          </span>
          <div>
            <p style="font-size: var(--font-size-sm); font-weight: var(--font-weight-medium); color: var(--sev-critical); margin-bottom: 4px;">
              Scan falló
            </p>
            <p style="font-size: var(--font-size-sm); color: var(--text-secondary);">
              {event.error.replace(/^invalid input: /, '').replace(/^internal error: /, '')}
            </p>
          </div>
        </div>
      </section>
    {/if}

    <!-- Resumen al completar OK -->
    {#if event?.status === 'done'}
      <section
        style="margin: 0 var(--space-5) var(--space-5); padding: var(--space-4); border-radius: var(--radius-lg);
               background: rgba(34, 197, 94, 0.06); border: 1px solid rgba(34, 197, 94, 0.25);"
      >
        <div style="display: flex; gap: var(--space-3); align-items: flex-start;">
          <span style="color: var(--sev-low); flex-shrink: 0; margin-top: 2px;">
            <Check size={16} />
          </span>
          <div style="flex: 1; min-width: 0;">
            <p style="font-size: var(--font-size-sm); font-weight: var(--font-weight-medium); color: var(--sev-low); margin-bottom: 4px;">
              Scan completado
            </p>
            <p style="font-size: var(--font-size-sm); color: var(--text-secondary);">
              {#if event.discovered_ports.length === 0}
                {event.note ?? 'No se detectaron puertos abiertos. El host puede tener todos los servicios cerrados, filtrados o sin servicios públicos expuestos.'}
              {:else}
                Se detectaron <strong style="color: var(--text-primary);">{event.discovered_ports.length}</strong>
                puerto{event.discovered_ports.length !== 1 ? 's' : ''} abierto{event.discovered_ports.length !== 1 ? 's' : ''}.
                Las vulnerabilidades aparecerán en el detalle del asset tras el análisis de riesgo.
              {/if}
            </p>
          </div>
        </div>
      </section>
    {/if}

    <!-- Stage timeline -->
    <section style="padding: 0 var(--space-5) var(--space-5);">
      <h3 style="font-size: var(--font-size-xs); font-weight: var(--font-weight-semibold); color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: var(--space-3);">
        Etapas
      </h3>
      <ol class="stage-list">
        {#each STAGES as stage, i (stage.key)}
          {@const st = stageStatus(i)}
          <li class="stage-item" data-status={st}>
            <span class="stage-marker">
              {#if st === 'done'}
                <Check size={12} />
              {:else if st === 'active'}
                <Loader size={12} />
              {:else}
                <span class="dot"></span>
              {/if}
            </span>
            <span class="stage-label" data-status={st}>{stage.label}</span>
          </li>
        {/each}
      </ol>
    </section>

    <!-- Discovered ports -->
    {#if event?.discovered_ports && event.discovered_ports.length > 0}
      <section style="padding: 0 var(--space-5) var(--space-5);">
        <h3 style="font-size: var(--font-size-xs); font-weight: var(--font-weight-semibold); color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: var(--space-3);">
          Puertos descubiertos ({event.discovered_ports.length})
        </h3>
        <div style="border: 1px solid var(--border); border-radius: var(--radius-md); overflow: hidden;">
          <table style="width: 100%; border-collapse: collapse; font-size: var(--font-size-sm);">
            <thead>
              <tr style="background: var(--bg-elevated);">
                <th style="padding: var(--space-2) var(--space-3); text-align: left; font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); color: var(--text-secondary);">Puerto</th>
                <th style="padding: var(--space-2) var(--space-3); text-align: left; font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); color: var(--text-secondary);">Proto</th>
                <th style="padding: var(--space-2) var(--space-3); text-align: left; font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); color: var(--text-secondary);">Servicio</th>
              </tr>
            </thead>
            <tbody>
              {#each event.discovered_ports as p}
                <tr style="border-top: 1px solid var(--border);">
                  <td style="padding: var(--space-2) var(--space-3); font-family: ui-monospace, monospace; color: var(--text-primary);">{p.port}</td>
                  <td style="padding: var(--space-2) var(--space-3); color: var(--text-secondary);">{p.protocol}</td>
                  <td style="padding: var(--space-2) var(--space-3); color: var(--text-secondary);">{p.service ?? '—'}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>
    {/if}

    <!-- Log tail -->
    {#if event?.log && event.log.length > 0}
      <section style="padding: 0 var(--space-5) var(--space-5);">
        <h3 style="font-size: var(--font-size-xs); font-weight: var(--font-weight-semibold); color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: var(--space-3);">
          Registro
        </h3>
        <pre class="log-tail" style="background: var(--bg-base); border: 1px solid var(--border); border-radius: var(--radius-md);">{event.log.slice(-12).join('\n')}</pre>
      </section>
    {/if}

    {#if connectionError && !isTerminal}
      <section style="padding: 0 var(--space-5) var(--space-5);">
        <p style="font-size: var(--font-size-xs); color: var(--text-muted);">
          Conexión interrumpida. Reintentando…
        </p>
      </section>
    {/if}
  </div>

  <!-- Footer con cierre cuando es terminal -->
  {#if isTerminal}
    <footer style="padding: var(--space-4) var(--space-5); border-top: 1px solid var(--border); display: flex; justify-content: flex-end; background: var(--bg-base);">
      <button
        onclick={onClose}
        style="padding: var(--space-2) var(--space-4); background: var(--accent); color: #fff; border: none; border-radius: var(--radius-md); font-size: var(--font-size-sm); font-weight: var(--font-weight-medium); cursor: pointer;"
      >
        Cerrar
      </button>
    </footer>
  {/if}
</div>

<style>
  .drawer-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 40;
  }
  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    height: 100vh;
    width: 440px;
    max-width: 90vw;
    z-index: 50;
    display: flex;
    flex-direction: column;
    animation: drawer-in 200ms ease-out;
  }
  @keyframes drawer-in {
    from { transform: translateX(100%); }
    to   { transform: translateX(0); }
  }
  .drawer-header {
    padding: var(--space-4) var(--space-5);
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-3);
  }
  .header-title {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
  }
  .header-title h2 { margin: 0; }
  .close-btn:hover { background: var(--bg-elevated); }
  .drawer-body {
    flex: 1;
    overflow-y: auto;
  }
  .stage-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: var(--space-2); }
  .stage-item { display: flex; align-items: center; gap: var(--space-3); }
  .stage-marker {
    flex-shrink: 0;
    width: 22px;
    height: 22px;
    border-radius: var(--radius-pill);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    color: var(--text-muted);
  }
  .stage-item[data-status="done"] .stage-marker {
    background: var(--sev-low);
    border-color: var(--sev-low);
    color: #fff;
  }
  .stage-item[data-status="active"] .stage-marker {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  .stage-marker .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-muted);
  }
  .stage-label {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    transition: color 200ms;
  }
  .stage-label[data-status="done"]   { color: var(--text-secondary); }
  .stage-label[data-status="active"] { color: var(--text-primary); font-weight: var(--font-weight-medium); }
  .log-tail {
    padding: var(--space-3);
    margin: 0;
    font-size: var(--font-size-xs);
    line-height: 1.5;
    color: var(--text-secondary);
    overflow-x: auto;
    white-space: pre-wrap;
    word-break: break-all;
    max-height: 180px;
    overflow-y: auto;
  }
</style>
