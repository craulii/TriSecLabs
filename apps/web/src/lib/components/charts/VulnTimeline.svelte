<script lang="ts">
  import EChart from './EChart.svelte';
  import type { Metric } from '$lib/types/models';
  import type { EChartsOption } from 'echarts';

  // Una serie de métricas por severidad (todas de la misma ventana temporal)
  let {
    critical = [],
    high     = [],
    medium   = [],
    low      = [],
    height   = '280px',
  }: {
    critical?: Metric[];
    high?:     Metric[];
    medium?:   Metric[];
    low?:      Metric[];
    height?:   string;
  } = $props();

  function toSeries(metrics: Metric[]) {
    return metrics.map(m => [m.period_start, m.value]);
  }

  function formatDate(iso: string) {
    return new Date(iso).toLocaleDateString('es', { month: 'short', day: 'numeric' });
  }

  const option: EChartsOption = $derived({
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'cross' },
      backgroundColor: 'var(--bg-surface)',
      borderColor:     'var(--border)',
      textStyle:       { color: 'var(--text-primary)', fontSize: 12 },
    },
    legend: {
      data: ['Critical', 'High', 'Medium', 'Low'],
      right: 0,
      itemWidth: 10,
      itemHeight: 10,
    },
    grid: { left: 40, right: 16, top: 40, bottom: 30, containLabel: false },
    xAxis: {
      type: 'category',
      // Extraer fechas del array más largo
      data: [...critical, ...high, ...medium, ...low]
        .map(m => m.period_start)
        .filter((v, i, a) => a.indexOf(v) === i)
        .sort()
        .map(formatDate),
      axisLine:  { lineStyle: { color: 'var(--border)' } },
      axisTick:  { show: false },
      axisLabel: { color: 'var(--text-secondary)', fontSize: 11 },
    },
    yAxis: {
      type: 'value',
      minInterval: 1,
      axisLine:  { show: false },
      axisTick:  { show: false },
      axisLabel: { color: 'var(--text-secondary)', fontSize: 11 },
      splitLine: { lineStyle: { color: 'var(--border)', type: 'dashed' } },
    },
    series: [
      {
        name: 'Critical',
        type: 'bar',
        stack: 'total',
        data: critical.map(m => m.value),
        itemStyle: { color: '#ef4444', borderRadius: [0, 0, 0, 0] },
      },
      {
        name: 'High',
        type: 'bar',
        stack: 'total',
        data: high.map(m => m.value),
        itemStyle: { color: '#f97316' },
      },
      {
        name: 'Medium',
        type: 'bar',
        stack: 'total',
        data: medium.map(m => m.value),
        itemStyle: { color: '#f59e0b' },
      },
      {
        name: 'Low',
        type: 'bar',
        stack: 'total',
        data: low.map(m => m.value),
        itemStyle: { color: '#22c55e', borderRadius: [3, 3, 0, 0] },
      },
    ],
  });
</script>

<EChart {option} {height} />
