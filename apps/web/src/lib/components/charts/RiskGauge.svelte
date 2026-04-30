<script lang="ts">
  import EChart from './EChart.svelte';
  import type { EChartsOption } from 'echarts';

  let { score = 0 }: { score: number } = $props();

  // Colormap: 4 segmentos que coinciden con risk_level del backend
  // [0-25) low, [25-50) medium, [50-75) high, [75-100] critical
  const option: EChartsOption = $derived({
    series: [{
      type: 'gauge',
      min:  0,
      max:  100,
      radius: '90%',
      startAngle: 210,
      endAngle:   -30,
      splitNumber: 4,
      axisLine: {
        lineStyle: {
          width: 16,
          color: [
            [0.25, '#22c55e'],  // low
            [0.50, '#f59e0b'],  // medium
            [0.75, '#f97316'],  // high
            [1.00, '#ef4444'],  // critical
          ],
        },
      },
      pointer: {
        itemStyle: { color: 'auto' },
        length: '60%',
        width: 5,
      },
      axisTick:   { show: false },
      splitLine:  { show: false },
      axisLabel:  { show: false },
      title: {
        offsetCenter: [0, '65%'],
        fontSize: 11,
        color: '#94a3b8',
      },
      detail: {
        valueAnimation: true,
        fontSize: 28,
        fontWeight: 'bold',
        offsetCenter: [0, '30%'],
        color: 'auto',
        formatter: '{value}',
      },
      data: [{ value: Math.round(score), name: 'Risk Score' }],
    }],
  });
</script>

<EChart {option} height="220px" />
