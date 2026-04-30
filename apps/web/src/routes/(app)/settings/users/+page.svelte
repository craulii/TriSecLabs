<script lang="ts">
  import { auth } from '$lib/stores/auth';
</script>

<svelte:head><title>Usuarios — TriSecLabs</title></svelte:head>

<div class="space-y-6">
  <!-- Banner próximamente -->
  <div class="rounded-xl p-5 flex gap-4 items-start"
       style="background: color-mix(in srgb, var(--accent) 8%, var(--bg-surface));
              border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);">
    <div class="flex-shrink-0 mt-0.5">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none"
           stroke="var(--accent)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
        <circle cx="9" cy="7" r="4"/>
        <path d="M23 21v-2a4 4 0 0 0-3-3.87"/>
        <path d="M16 3.13a4 4 0 0 1 0 7.75"/>
      </svg>
    </div>
    <div>
      <p class="font-semibold text-sm mb-1" style="color: var(--accent);">
        Gestión de usuarios — Próximamente
      </p>
      <p class="text-sm" style="color: var(--text-secondary);">
        Esta sección permitirá crear y gestionar usuarios del tenant, asignar roles
        (admin / analyst) y revocar accesos.
      </p>
    </div>
  </div>

  <!-- Usuario activo -->
  <div class="rounded-xl p-5"
       style="background: var(--bg-surface); border: 1px solid var(--border);">
    <h2 class="text-sm font-semibold mb-4" style="color: var(--text-primary);">
      Sesión actual
    </h2>
    <dl class="space-y-3">
      <div class="flex gap-4">
        <dt class="text-xs w-24 flex-shrink-0 pt-0.5" style="color: var(--text-muted);">User ID</dt>
        <dd class="text-sm font-mono break-all" style="color: var(--text-primary);">
          {$auth.userId ?? '—'}
        </dd>
      </div>
      <div class="flex gap-4">
        <dt class="text-xs w-24 flex-shrink-0 pt-0.5" style="color: var(--text-muted);">Rol</dt>
        <dd>
          <span class="text-xs px-2 py-0.5 rounded font-medium"
                style="background: color-mix(in srgb, var(--accent) 15%, transparent);
                       color: var(--accent);">
            {$auth.role ?? '—'}
          </span>
        </dd>
      </div>
      <div class="flex gap-4">
        <dt class="text-xs w-24 flex-shrink-0 pt-0.5" style="color: var(--text-muted);">Tenant</dt>
        <dd class="text-sm font-mono break-all" style="color: var(--text-primary);">
          {$auth.tenantId ?? '—'}
        </dd>
      </div>
    </dl>
  </div>

  <!-- Roles disponibles -->
  <div class="rounded-xl p-5"
       style="background: var(--bg-surface); border: 1px solid var(--border);">
    <h2 class="text-sm font-semibold mb-3" style="color: var(--text-primary);">Roles del sistema</h2>
    <div class="space-y-2">
      {#each [
        { role: 'admin',   desc: 'Acceso completo. Puede gestionar tenants, usuarios, assets y ver todas las vulnerabilidades.' },
        { role: 'analyst', desc: 'Acceso de lectura y operación. Puede lanzar scans e informes, pero no gestionar usuarios.' },
      ] as r}
        <div class="flex gap-3 items-start py-2 px-3 rounded"
             style="background: var(--bg-elevated);">
          <span class="text-xs font-mono px-1.5 py-0.5 rounded flex-shrink-0 mt-0.5"
                style="background: color-mix(in srgb, var(--accent) 15%, transparent);
                       color: var(--accent);">
            {r.role}
          </span>
          <p class="text-xs" style="color: var(--text-secondary);">{r.desc}</p>
        </div>
      {/each}
    </div>
  </div>
</div>
