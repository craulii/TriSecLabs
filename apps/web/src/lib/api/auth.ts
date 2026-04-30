import { api }          from './client';
import type { LoginResponse } from '$lib/types/models';

export const authApi = {
  login(tenantSlug: string, email: string, password: string) {
    return api.post<LoginResponse>('/auth/login', {
      tenant_slug: tenantSlug,
      email,
      password,
    });
  },
};
