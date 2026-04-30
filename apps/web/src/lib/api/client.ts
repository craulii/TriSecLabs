import { get }  from 'svelte/store';
import { goto } from '$app/navigation';
import { auth }  from '$lib/stores/auth';

// En prod Axum sirve la app desde el mismo origin → no hay prefijo especial.
// En dev Vite proxia /api → localhost:3000, tampoco se necesita host explícito.
const BASE = import.meta.env.VITE_API_BASE ?? '/api';

// ─── Error tipado ────────────────────────────────────────────────────────────

export class ApiError extends Error {
  constructor(
    public readonly status:  number,
    public readonly message: string,
    public readonly body?:   unknown,
  ) {
    super(message);
    this.name = 'ApiError';
  }

  get isUnauthorized() { return this.status === 401; }
  get isForbidden()    { return this.status === 403; }
  get isNotFound()     { return this.status === 404; }
  get isServer()       { return this.status >= 500; }
}

// ─── Request base ─────────────────────────────────────────────────────────────

async function request<T>(
  path: string,
  options: RequestInit = {},
): Promise<T> {
  const token = get(auth).token;

  const headers: HeadersInit = {
    'Content-Type': 'application/json',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...(options.headers ?? {}),
  };

  const res = await fetch(`${BASE}${path}`, { ...options, headers });

  if (res.status === 401) {
    auth.logout();
    await goto('/login');
    throw new ApiError(401, 'Sesión expirada');
  }

  if (!res.ok) {
    let errorMsg = res.statusText;
    let body: unknown;
    try {
      body = await res.json();
      if (body && typeof body === 'object' && 'error' in body) {
        errorMsg = (body as { error: string }).error;
      }
    } catch { /* body no es JSON */ }
    throw new ApiError(res.status, errorMsg, body);
  }

  // 204 No Content — retornar undefined casteado a T
  if (res.status === 204) return undefined as unknown as T;

  return res.json() as Promise<T>;
}

// ─── API client ───────────────────────────────────────────────────────────────

export const api = {
  get<T>(path: string, params?: Record<string, string | number>): Promise<T> {
    const url = params
      ? `${path}?${new URLSearchParams(
          Object.fromEntries(
            Object.entries(params).map(([k, v]) => [k, String(v)])
          )
        )}`
      : path;
    return request<T>(url);
  },

  post<T>(path: string, body: unknown): Promise<T> {
    return request<T>(path, {
      method: 'POST',
      body:   JSON.stringify(body),
    });
  },

  patch<T>(path: string, body: unknown): Promise<T> {
    return request<T>(path, {
      method: 'PATCH',
      body:   JSON.stringify(body),
    });
  },

  delete<T>(path: string): Promise<T> {
    return request<T>(path, { method: 'DELETE' });
  },
};
