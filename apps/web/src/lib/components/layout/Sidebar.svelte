<script lang="ts">
  import { page }             from '$app/stores';
  import { auth, isAdmin }    from '$lib/stores/auth';
  import { sidebarCollapsed } from '$lib/stores/ui';

  const navItems = [
    { href: '/dashboard',        label: 'Dashboard',        icon: 'grid' },
    { href: '/assets',           label: 'Assets',           icon: 'server' },
    { href: '/vulnerabilities',  label: 'Vulnerabilidades', icon: 'shield' },
    { href: '/reports',          label: 'Informes LLM',     icon: 'file' },
  ];

  const adminItems = [
    { href: '/settings/tenants', label: 'Tenants',  icon: 'building' },
    { href: '/settings/users',   label: 'Usuarios', icon: 'users' },
  ];

  let current  = $derived($page.url.pathname);
  let collapsed = $derived($sidebarCollapsed);

  function isActive(href: string): boolean {
    return current === href || (href !== '/dashboard' && current.startsWith(href));
  }
</script>

{#snippet NavIcon(name: string)}
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none"
       stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    {#if name === 'grid'}
      <rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/>
      <rect x="14" y="14" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/>
    {:else if name === 'server'}
      <rect x="2" y="2" width="20" height="8" rx="2"/><rect x="2" y="14" width="20" height="8" rx="2"/>
      <line x1="6" y1="6" x2="6.01" y2="6"/><line x1="6" y1="18" x2="6.01" y2="18"/>
    {:else if name === 'shield'}
      <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
    {:else if name === 'file'}
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
      <polyline points="14 2 14 8 20 8"/>
    {:else if name === 'building'}
      <rect x="3" y="9" width="18" height="12"/><path d="M3 9l9-7 9 7"/>
      <line x1="9" y1="22" x2="9" y2="12"/><line x1="15" y1="22" x2="15" y2="12"/>
    {:else if name === 'users'}
      <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
      <circle cx="9" cy="7" r="4"/>
      <path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>
    {:else if name === 'log-out'}
      <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/>
      <polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/>
    {:else}
      <circle cx="12" cy="12" r="10"/>
    {/if}
  </svg>
{/snippet}

<aside
  class="flex flex-col h-full transition-[width] duration-200"
  style="
    width: {collapsed ? '56px' : '220px'};
    background: var(--bg-surface);
    border-right: 1px solid var(--border);
    flex-shrink: 0;
  "
>
  <!-- Logo -->
  <div class="flex items-center gap-3 px-4 py-5" style="border-bottom: 1px solid var(--border);">
    <div class="w-7 h-7 rounded flex items-center justify-center flex-shrink-0"
         style="background: var(--accent);">
      <span class="text-white font-bold text-xs">T</span>
    </div>
    {#if !collapsed}
      <span class="font-semibold text-sm tracking-tight" style="color: var(--text-primary);">
        TriSecLabs
      </span>
    {/if}
  </div>

  <!-- Navegación principal -->
  <nav class="flex-1 py-3 overflow-y-auto">
    <ul class="space-y-0.5 px-2">
      {#each navItems as item}
        <li>
          <a
            href={item.href}
            class="flex items-center gap-3 px-2 py-2 rounded text-sm transition-colors"
            style="
              color: {isActive(item.href) ? 'var(--accent)' : 'var(--text-secondary)'};
              background: {isActive(item.href) ? 'color-mix(in srgb, var(--accent) 12%, transparent)' : 'transparent'};
            "
          >
            {@render NavIcon(item.icon)}
            {#if !collapsed}
              <span>{item.label}</span>
            {/if}
          </a>
        </li>
      {/each}
    </ul>

    {#if $isAdmin}
      <div class="mt-4 px-4 mb-1">
        {#if !collapsed}
          <span class="text-xs uppercase tracking-wider" style="color: var(--text-muted);">
            Administración
          </span>
        {/if}
      </div>
      <ul class="space-y-0.5 px-2">
        {#each adminItems as item}
          <li>
            <a
              href={item.href}
              class="flex items-center gap-3 px-2 py-2 rounded text-sm transition-colors"
              style="
                color: {isActive(item.href) ? 'var(--accent)' : 'var(--text-secondary)'};
                background: {isActive(item.href) ? 'color-mix(in srgb, var(--accent) 12%, transparent)' : 'transparent'};
              "
            >
              {@render NavIcon(item.icon)}
              {#if !collapsed}
                <span>{item.label}</span>
              {/if}
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </nav>

  <!-- Footer: usuario + logout -->
  <div class="px-3 py-3" style="border-top: 1px solid var(--border);">
    {#if !collapsed}
      <div class="flex items-center gap-2 px-1 mb-2">
        <div class="w-6 h-6 rounded-full flex items-center justify-center text-xs font-semibold flex-shrink-0"
             style="background: var(--accent); color: white;">
          {($auth.role?.[0] ?? '?').toUpperCase()}
        </div>
        <div class="min-w-0">
          <p class="text-xs font-medium truncate" style="color: var(--text-primary);">
            {$auth.role}
          </p>
        </div>
      </div>
    {/if}
    <button
      onclick={auth.logout}
      class="flex items-center gap-3 w-full px-2 py-2 rounded text-sm transition-colors"
      style="color: var(--text-muted);"
      title="Cerrar sesión"
    >
      {@render NavIcon('log-out')}
      {#if !collapsed}
        <span>Salir</span>
      {/if}
    </button>
  </div>
</aside>
