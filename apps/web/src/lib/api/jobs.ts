import { get } from 'svelte/store';
import { api } from './client';
import { auth } from '$lib/stores/auth';
import type { JobProgressEvent } from '$lib/types/models';

const BASE = import.meta.env.VITE_API_BASE ?? '/api';

export const jobsApi = {
  /** Snapshot puntual del job. Usado como fallback si EventSource falla. */
  getSnapshot(id: string) {
    return api.get<JobProgressEvent>(`/jobs/${id}`);
  },

  /**
   * Abre un EventSource hacia el endpoint SSE del job.
   * Devuelve la instancia para que el caller pueda llamar `.close()` en cleanup.
   * Cierra automáticamente al recibir un evento con status terminal.
   */
  stream(
    id: string,
    onEvent: (e: JobProgressEvent) => void,
    onError?: (err: Event) => void,
  ): EventSource {
    const token = get(auth).token;
    const url = `${BASE}/jobs/${id}/stream?token=${encodeURIComponent(token ?? '')}`;
    const es = new EventSource(url);

    es.onmessage = (m) => {
      try {
        const ev = JSON.parse(m.data) as JobProgressEvent;
        onEvent(ev);
        if (ev.status === 'done' || ev.status === 'failed') {
          es.close();
        }
      } catch {
        // Mensaje malformado — ignorar
      }
    };

    if (onError) es.onerror = onError;

    return es;
  },
};
