<script lang="ts">
  import { goto }       from '$app/navigation';
  import { authApi }    from '$lib/api/auth';
  import { auth }       from '$lib/stores/auth';
  import { ApiError }   from '$lib/api/client';

  let tenantSlug = $state('');
  let email      = $state('');
  let password   = $state('');
  let error      = $state<string | null>(null);
  let loading    = $state(false);

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    error   = null;
    loading = true;
    try {
      const resp = await authApi.login(tenantSlug, email, password);
      auth.login({
        token:    resp.token,
        userId:   resp.user_id,
        tenantId: resp.tenant_id,
        role:     resp.role,
      });
      await goto('/dashboard');
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Error al conectar con el servidor';
    } finally {
      loading = false;
    }
  }
</script>

<svelte:head>
  <title>Iniciar sesión — TriSecLabs</title>
</svelte:head>

<div class="min-h-screen flex items-center justify-center px-4"
     style="background: var(--bg-base);">
  <div class="w-full" style="max-width: 380px;">

    <!-- Logo -->
    <div class="flex items-center justify-center gap-3 mb-8">
      <div class="w-9 h-9 rounded-lg flex items-center justify-center"
           style="background: var(--accent);">
        <span class="text-white font-bold text-base">T</span>
      </div>
      <span class="font-bold text-xl" style="color: var(--text-primary);">TriSecLabs</span>
    </div>

    <!-- Card -->
    <div class="rounded-xl p-6"
         style="background: var(--bg-surface); border: 1px solid var(--border);">
      <h1 class="text-lg font-semibold mb-5" style="color: var(--text-primary);">
        Iniciar sesión
      </h1>

      {#if error}
        <div class="mb-4 px-3 py-2.5 rounded-lg text-sm"
             style="background: color-mix(in srgb, #ef4444 10%, transparent); color: #ef4444; border: 1px solid #ef4444;">
          {error}
        </div>
      {/if}

      <form onsubmit={handleSubmit} class="space-y-4">
        <div>
          <label class="block text-xs font-medium mb-1.5"
                 style="color: var(--text-secondary);">
            Organización
          </label>
          <input
            type="text"
            bind:value={tenantSlug}
            placeholder="mi-empresa"
            required
            autocomplete="organization"
            class="w-full px-3 py-2 rounded-lg text-sm outline-none transition-colors"
            style="
              background: var(--bg-elevated);
              border: 1px solid var(--border);
              color: var(--text-primary);
            "
          />
        </div>

        <div>
          <label class="block text-xs font-medium mb-1.5"
                 style="color: var(--text-secondary);">
            Email
          </label>
          <input
            type="email"
            bind:value={email}
            placeholder="usuario@empresa.com"
            required
            autocomplete="email"
            class="w-full px-3 py-2 rounded-lg text-sm outline-none transition-colors"
            style="
              background: var(--bg-elevated);
              border: 1px solid var(--border);
              color: var(--text-primary);
            "
          />
        </div>

        <div>
          <label class="block text-xs font-medium mb-1.5"
                 style="color: var(--text-secondary);">
            Contraseña
          </label>
          <input
            type="password"
            bind:value={password}
            required
            autocomplete="current-password"
            class="w-full px-3 py-2 rounded-lg text-sm outline-none transition-colors"
            style="
              background: var(--bg-elevated);
              border: 1px solid var(--border);
              color: var(--text-primary);
            "
          />
        </div>

        <button
          type="submit"
          disabled={loading}
          class="w-full py-2.5 rounded-lg text-sm font-medium transition-opacity"
          style="background: var(--accent); color: white; opacity: {loading ? '0.7' : '1'};"
        >
          {loading ? 'Iniciando sesión…' : 'Iniciar sesión'}
        </button>
      </form>
    </div>
  </div>
</div>
