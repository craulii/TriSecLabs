import { api } from './client';
import type { Vulnerability, VulnStatus } from '$lib/types/models';

export const vulnsApi = {
  listForTarget(targetId: string) {
    return api.get<Vulnerability[]>(`/targets/${targetId}/vulnerabilities`);
  },

  // Listado global con paginación (implementar en Axum cuando haga falta)
  list(params?: { page?: number; limit?: number; severity?: string; status?: string }) {
    return api.get<Vulnerability[]>('/vulnerabilities', params as Record<string, string | number>);
  },

  updateStatus(id: string, status: VulnStatus, note?: string) {
    return api.patch<void>(`/vulnerabilities/${id}/status`, { status, note });
  },
};
