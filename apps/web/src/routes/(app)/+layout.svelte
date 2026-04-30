<script lang="ts">
  import Sidebar from '$lib/components/layout/Sidebar.svelte';
  import Topbar  from '$lib/components/layout/Topbar.svelte';

  let { children } = $props();
</script>

<div class="flex h-screen overflow-hidden" style="background: var(--bg-base);">
  <Sidebar />
  <div class="flex flex-col flex-1 min-w-0 overflow-hidden">
    <Topbar />
    <main class="flex-1 overflow-y-auto p-6">
      {@render children()}
    </main>
  </div>
</div>
