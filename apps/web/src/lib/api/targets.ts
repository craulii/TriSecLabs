import { api }       from './client';
import type { ScanTarget, JobAccepted, ExposedPort } from '$lib/types/models';

export interface LatestJobSummary {
  id:           string;
  status:       'pending' | 'running' | 'done' | 'failed';
  attempts:     number;
  error:        string | null;
  progress:     number | null;
  current_step: string | null;
  stats_json:   Record<string, unknown>;
  created_at:   string;
  updated_at:   string;
}

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

  update(id: string, payload: { name: string; value: string }) {
    return api.patch<ScanTarget>(`/vendors/${id}`, payload);
  },

  delete(id: string) {
    return api.delete(`/vendors/${id}`);
  },

  getLatestJob(id: string) {
    return api.get<LatestJobSummary>(`/vendors/${id}/job`);
  },

  listPorts(id: string) {
    return api.get<ExposedPort[]>(`/targets/${id}/ports`);
  },
};
