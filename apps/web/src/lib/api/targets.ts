import { api }       from './client';
import type { ScanTarget, JobAccepted, ExposedPort } from '$lib/types/models';

export const targetsApi = {
  list() {
    return api.get<ScanTarget[]>('/vendors');
  },

  get(id: string) {
    return api.get<ScanTarget>(`/vendors/${id}`);
  },

  create(payload: { kind: ScanTarget['kind']; name: string; value: string }) {
    return api.post<ScanTarget>('/vendors', payload);
  },

  enqueueScan(id: string) {
    return api.post<JobAccepted>(`/vendors/${id}/scan`, {});
  },

  enqueueLlmReport(id: string) {
    return api.post<JobAccepted>(`/vendors/${id}/analyze`, {});
  },

  listPorts(id: string) {
    return api.get<ExposedPort[]>(`/targets/${id}/ports`);
  },
};
