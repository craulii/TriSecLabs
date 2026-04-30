<script lang="ts">
  let { size = 16, strokeWidth = 2, class: className = '' }: {
    size?: number; strokeWidth?: number; class?: string;
  } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width={strokeWidth}
     stroke-linecap="round" stroke-linejoin="round" class={className}>
  <polygon points="6 4 20 12 6 20 6 4" fill="currentColor" stroke="none" />
</svg>
