<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { EChartsOption }  from 'echarts';

  // ─── Props ──────────────────────────────────────────────────────────────────

  let {
    option,
    height  = '300px',
    loading = false,
  }: {
    option:   EChartsOption;
    height?:  string;
    loading?: boolean;
  } = $props();

  // ─── Internos ────────────────────────────────────────────────────────────────

  let container: HTMLDivElement;
  let instance:  import('echarts').ECharts | undefined;
  let resizeObs: ResizeObserver | undefined;

  onMount(async () => {
    // Importación dinámica: ECharts no es SSR-safe (accede a window/document).
    // Con adapter-static (SPA mode) el código nunca corre en Node, pero el
    // import dinámico es el patrón correcto para evitar problemas si alguna vez
    // se habilita SSR parcial.
    const echarts = await import('echarts');

    // Tema que coincide con los design tokens del app.css
    echarts.registerTheme('tsl-dark', {
      backgroundColor: 'transparent',
      textStyle: { color: '#94a3b8' },       // --text-secondary
      axisPointer: { lineStyle: { color: '#475569' } },
      splitLine: { lineStyle: { color: '#334155' } },
      legend: { textStyle: { color: '#94a3b8' } },
    });
    echarts.registerTheme('tsl-light', {
      backgroundColor: 'transparent',
      textStyle: { color: '#475569' },
      axisPointer: { lineStyle: { color: '#e2e8f0' } },
      splitLine: { lineStyle: { color: '#f1f5f9' } },
    });

    const themeName = document.documentElement.getAttribute('data-theme') === 'light'
      ? 'tsl-light'
      : 'tsl-dark';

    instance = echarts.init(container, themeName, { renderer: 'svg' });
    instance.setOption(option);

    // ResizeObserver: redimensiona el chart cuando cambia el contenedor
    resizeObs = new ResizeObserver(() => instance?.resize());
    resizeObs.observe(container);
  });

  // Reactivo: actualizar opción cuando cambia desde fuera
  // notMerge: true — reemplaza la opción completa (evita merge de series previas)
  $effect(() => {
    if (instance) {
      instance.setOption(option, { notMerge: true, lazyUpdate: true });
    }
  });

  $effect(() => {
    if (instance) {
      loading ? instance.showLoading() : instance.hideLoading();
    }
  });

  onDestroy(() => {
    resizeObs?.disconnect();
    instance?.dispose();
  });
</script>

<div bind:this={container} style="height: {height}; width: 100%;"></div>
