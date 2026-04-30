<script lang="ts">
  let { size = 16, strokeWidth = 2, class: className = '' }: {
    size?: number; strokeWidth?: number; class?: string;
  } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width={strokeWidth}
     stroke-linecap="round" stroke-linejoin="round" class={className}>
  <path d="M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.5.5 0 0 1-.96 0L9.24 2.18a.5.5 0 0 0-.96 0l-2.35 8.36A2 2 0 0 1 4 12H2" />
</svg>
