<svelte:head><title>Informes LLM — TriSecLabs</title></svelte:head>

<div class="space-y-6">
  <!-- Banner de estado -->
  <div class="rounded-xl p-5 flex gap-4 items-start"
       style="background: color-mix(in srgb, #f59e0b 10%, var(--bg-surface));
              border: 1px solid color-mix(in srgb, #f59e0b 40%, transparent);">
    <div class="flex-shrink-0 mt-0.5">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none"
           stroke="#f59e0b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
        <line x1="12" y1="9" x2="12" y2="13"/>
        <line x1="12" y1="17" x2="12.01" y2="17"/>
      </svg>
    </div>
    <div>
      <p class="font-semibold text-sm mb-1" style="color: #f59e0b;">
        Módulo LLM no disponible
      </p>
      <p class="text-sm" style="color: var(--text-secondary);">
        El módulo de informes requiere un servidor LLM local activo (llama.cpp).
        Los trabajos se encolan correctamente desde la página de Assets, pero no se procesarán hasta que el servidor esté corriendo.
      </p>
    </div>
  </div>

  <!-- Cómo activarlo -->
  <div class="rounded-xl p-5"
       style="background: var(--bg-surface); border: 1px solid var(--border);">
    <h2 class="text-sm font-semibold mb-3" style="color: var(--text-primary);">
      Cómo activar el servidor LLM
    </h2>
    <p class="text-sm mb-3" style="color: var(--text-secondary);">
      Ejecuta el siguiente comando en la raíz del proyecto para levantar el servidor LLM local:
    </p>
    <pre class="text-xs px-4 py-3 rounded-lg overflow-x-auto"
         style="background: var(--bg-elevated); color: var(--text-primary); border: 1px solid var(--border);">docker compose --profile llm up -d</pre>
    <p class="text-xs mt-3" style="color: var(--text-muted);">
      Una vez activo, los trabajos de tipo <code style="color: var(--accent);">llm_report</code> en cola serán procesados automáticamente por el worker.
    </p>
  </div>

  <!-- Qué incluirá esta sección -->
  <div class="rounded-xl p-5"
       style="background: var(--bg-surface); border: 1px solid var(--border);">
    <h2 class="text-sm font-semibold mb-3" style="color: var(--text-primary);">
      Próximamente
    </h2>
    <ul class="space-y-2 text-sm" style="color: var(--text-secondary);">
      {#each [
        'Historial de informes generados por asset',
        'Visualización del contenido del informe (Markdown)',
        'Comparativa de informes entre fechas',
        'Exportación a PDF',
      ] as item}
        <li class="flex items-center gap-2">
          <span style="color: var(--text-muted);">–</span>
          {item}
        </li>
      {/each}
    </ul>
  </div>
</div>
