<script lang="ts">
  let {
    value = 0,
    label = '',
    indeterminate = false,
    size = 'md',
    color = 'var(--accent)',
  }: {
    value?: number;
    label?: string;
    indeterminate?: boolean;
    size?: 'sm' | 'md' | 'lg';
    color?: string;
  } = $props();

  const heights = { sm: '4px', md: '8px', lg: '12px' };
  const heightVar = $derived(heights[size]);
  const clamped = $derived(Math.max(0, Math.min(100, value)));
</script>

<div class="progress-wrapper">
  {#if label}
    <div class="progress-label">
      <span style="color: var(--text-secondary); font-size: var(--font-size-sm);">{label}</span>
      {#if !indeterminate}
        <span style="color: var(--text-muted); font-size: var(--font-size-xs); font-variant-numeric: tabular-nums;">
          {Math.round(clamped)}%
        </span>
      {/if}
    </div>
  {/if}

  <div
    class="progress-track"
    style="height: {heightVar}; background: var(--bg-elevated); border-radius: var(--radius-pill);"
    role="progressbar"
    aria-valuenow={indeterminate ? undefined : Math.round(clamped)}
    aria-valuemin="0"
    aria-valuemax="100"
  >
    {#if indeterminate}
      <div class="progress-bar indeterminate" style="background: {color}; height: 100%;"></div>
    {:else}
      <div
        class="progress-bar"
        style="width: {clamped}%; background: {color}; height: 100%;"
      ></div>
    {/if}
  </div>
</div>

<style>
  .progress-wrapper { display: flex; flex-direction: column; gap: 0.5rem; }
  .progress-label   { display: flex; justify-content: space-between; align-items: baseline; }
  .progress-track   { width: 100%; overflow: hidden; position: relative; }
  .progress-bar     { border-radius: var(--radius-pill); transition: width 300ms ease-out; }

  .progress-bar.indeterminate {
    width: 35%;
    animation: shimmer 1.4s ease-in-out infinite;
  }
  @keyframes shimmer {
    0%   { transform: translateX(-100%); }
    50%  { transform: translateX(150%); }
    100% { transform: translateX(150%); }
  }
</style>
