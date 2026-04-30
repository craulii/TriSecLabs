import { writable, derived } from 'svelte/store';
import { browser }           from '$app/environment';

// ─── Tema ────────────────────────────────────────────────────────────────────

type Theme = 'dark' | 'light';

function createThemeStore() {
  const stored = browser
    ? (localStorage.getItem('tsl_theme') as Theme | null)
    : null;

  // Detectar preferencia del sistema si no hay valor guardado
  const system: Theme =
    browser && window.matchMedia('(prefers-color-scheme: light)').matches
      ? 'light'
      : 'dark';

  const { subscribe, set, update } = writable<Theme>(stored ?? system);

  return {
    subscribe,
    toggle() {
      update(t => {
        const next = t === 'dark' ? 'light' : 'dark';
        if (browser) {
          localStorage.setItem('tsl_theme', next);
          document.documentElement.setAttribute('data-theme', next);
        }
        return next;
      });
    },
    set(theme: Theme) {
      if (browser) {
        localStorage.setItem('tsl_theme', theme);
        document.documentElement.setAttribute('data-theme', theme);
      }
      set(theme);
    },
  };
}

export const theme = createThemeStore();

// ─── Sidebar ─────────────────────────────────────────────────────────────────

export const sidebarCollapsed = writable<boolean>(false);

// ─── Notificaciones (toast) ───────────────────────────────────────────────────

export type ToastKind = 'info' | 'success' | 'warning' | 'error';

export interface Toast {
  id:      number;
  kind:    ToastKind;
  message: string;
}

let _toastId = 0;

function createToastStore() {
  const { subscribe, update } = writable<Toast[]>([]);

  function push(kind: ToastKind, message: string, durationMs = 4000) {
    const id = ++_toastId;
    update(ts => [...ts, { id, kind, message }]);
    if (durationMs > 0) {
      setTimeout(() => dismiss(id), durationMs);
    }
    return id;
  }

  function dismiss(id: number) {
    update(ts => ts.filter(t => t.id !== id));
  }

  return {
    subscribe,
    info:    (msg: string) => push('info', msg),
    success: (msg: string) => push('success', msg),
    warning: (msg: string) => push('warning', msg),
    error:   (msg: string, duration?: number) => push('error', msg, duration ?? 6000),
    dismiss,
  };
}

export const toasts = createToastStore();
