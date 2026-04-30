<script lang="ts">
  let { size = 16, strokeWidth = 2, class: className = '' }: {
    size?: number; strokeWidth?: number; class?: string;
  } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width={strokeWidth}
     stroke-linecap="round" stroke-linejoin="round" class={className}>
  <path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z" />
  <path d="m15 5 4 4" />
</svg>
