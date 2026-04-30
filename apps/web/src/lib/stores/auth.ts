import { writable, derived } from 'svelte/store';
import { browser }           from '$app/environment';

// ─── Tipos ───────────────────────────────────────────────────────────────────

export interface AuthState {
  token:    string | null;
  userId:   string | null;
  tenantId: string | null;
  role:     'admin' | 'analyst' | null;
}

const INITIAL: AuthState = {
  token: null, userId: null, tenantId: null, role: null,
};

// ─── Detección de plataforma ─────────────────────────────────────────────────
// En Tauri: el WebView tiene window.__TAURI_INTERNALS__ inyectado.
// En browser real: sessionStorage. En Tauri: memory-only (el proceso sidecar
// se reinicia con la app de todas formas, el login aplica igual).
// Ventaja de memory-only en Tauri: el token nunca toca ningún storage
// accesible por extensiones u otras pestañas (no existen en Tauri de todos modos,
// pero es el modelo correcto para una app de seguridad).

function isTauriRuntime(): boolean {
  if (!browser) return false;
  return typeof (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ !== 'undefined';
}

// ─── Persistencia ────────────────────────────────────────────────────────────

const STORAGE_KEY = 'tsl_session';

function load(): AuthState {
  if (!browser || isTauriRuntime()) return INITIAL;
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as AuthState) : INITIAL;
  } catch {
    return INITIAL;
  }
}

function save(state: AuthState) {
  if (!browser || isTauriRuntime()) return;
  if (state.token) {
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } else {
    sessionStorage.removeItem(STORAGE_KEY);
  }
}

// ─── Store ───────────────────────────────────────────────────────────────────

function createAuthStore() {
  const { subscribe, set, update } = writable<AuthState>(load());

  return {
    subscribe,

    login(state: AuthState) {
      save(state);
      set(state);
    },

    logout() {
      save(INITIAL);
      set(INITIAL);
    },

    refreshToken(newToken: string) {
      update(s => {
        const next = { ...s, token: newToken };
        save(next);
        return next;
      });
    },
  };
}

export const auth = createAuthStore();

// ─── Derivados ───────────────────────────────────────────────────────────────

export const isAuthenticated = derived(auth, $a => $a.token !== null);
export const isAdmin         = derived(auth, $a => $a.role === 'admin');
