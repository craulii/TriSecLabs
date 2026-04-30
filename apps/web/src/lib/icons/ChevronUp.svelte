<script lang="ts">
  let { size = 16, strokeWidth = 2, class: className = '' }: {
    size?: number; strokeWidth?: number; class?: string;
  } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width={strokeWidth}
     stroke-linecap="round" stroke-linejoin="round" class={className}>
  <path d="m18 15-6-6-6 6" />
</svg>
