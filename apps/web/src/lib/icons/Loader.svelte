<script lang="ts">
  let { size = 16, strokeWidth = 2, class: className = '' }: {
    size?: number; strokeWidth?: number; class?: string;
  } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width={strokeWidth}
     stroke-linecap="round" stroke-linejoin="round" class="loader-spin {className}">
  <path d="M21 12a9 9 0 1 1-6.219-8.56" />
</svg>

<style>
  .loader-spin { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
