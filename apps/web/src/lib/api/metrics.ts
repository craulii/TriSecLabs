import { api } from './client';
import type { Metric, MetricKind } from '$lib/types/models';

export const metricsApi = {
  history(targetId: string, kind: MetricKind, limit = 30) {
    return api.get<Metric[]>(`/targets/${targetId}/metrics/${kind}`, { limit });
  },

  latest(targetId: string) {
    // Carga en paralelo las métricas más importantes para el dashboard de un target
    return Promise.all([
      metricsApi.history(targetId, 'risk_score',           1),
      metricsApi.history(targetId, 'vuln_count_critical',  1),
      metricsApi.history(targetId, 'vuln_count_high',      1),
      metricsApi.history(targetId, 'exposed_port_count',   1),
    ]).then(([riskScore, critical, high, ports]) => ({
      riskScore:    riskScore[0]?.value ?? 0,
      critical:     critical[0]?.value  ?? 0,
      high:         high[0]?.value      ?? 0,
      exposedPorts: ports[0]?.value     ?? 0,
    }));
  },
};
