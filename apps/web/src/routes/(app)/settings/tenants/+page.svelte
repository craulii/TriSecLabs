<script lang="ts">
  import { auth } from '$lib/stores/auth';
</script>

<svelte:head><title>Tenants — TriSecLabs</title></svelte:head>

<div class="space-y-6">
  <!-- Banner próximamente -->
  <div class="rounded-xl p-5 flex gap-4 items-start"
       style="background: color-mix(in srgb, var(--accent) 8%, var(--bg-surface));
              border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);">
    <div class="flex-shrink-0 mt-0.5">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none"
           stroke="var(--accent)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="3" y="9" width="18" height="12"/>
        <path d="M3 9l9-7 9 7"/>
        <line x1="9" y1="22" x2="9" y2="12"/>
        <line x1="15" y1="22" x2="15" y2="12"/>
      </svg>
    </div>
    <div>
      <p class="font-semibold text-sm mb-1" style="color: var(--accent);">
        Gestión de tenants — Próximamente
      </p>
      <p class="text-sm" style="color: var(--text-secondary);">
        Esta sección permitirá crear, editar y eliminar tenants, gestionar sus configuraciones
        y administrar el acceso multi-organización.
      </p>
    </div>
  </div>

  <!-- Tenant activo -->
  <div class="rounded-xl p-5"
       style="background: var(--bg-surface); border: 1px solid var(--border);">
    <h2 class="text-sm font-semibold mb-4" style="color: var(--text-primary);">
      Tenant activo
    </h2>
    <dl class="space-y-3">
      <div class="flex gap-4">
        <dt class="text-xs w-24 flex-shrink-0 pt-0.5" style="color: var(--text-muted);">Tenant ID</dt>
        <dd class="text-sm font-mono break-all" style="color: var(--text-primary);">
          {$auth.tenantId ?? '—'}
        </dd>
      </div>
      <div class="flex gap-4">
        <dt class="text-xs w-24 flex-shrink-0 pt-0.5" style="color: var(--text-muted);">User ID</dt>
        <dd class="text-sm font-mono break-all" style="color: var(--text-primary);">
          {$auth.userId ?? '—'}
        </dd>
      </div>
      <div class="flex gap-4">
        <dt class="text-xs w-24 flex-shrink-0 pt-0.5" style="color: var(--text-muted);">Rol</dt>
        <dd class="text-sm" style="color: var(--text-primary);">
          {$auth.role ?? '—'}
        </dd>
      </div>
    </dl>
  </div>
</div>
