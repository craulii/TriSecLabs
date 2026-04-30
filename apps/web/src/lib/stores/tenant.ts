import { writable, derived } from 'svelte/store';
import { auth }               from './auth';

// El tenantId activo viene del JWT — no es configurable por el usuario.
// Este store es un derived que expone el tenant activo sin tener que
// acceder al store de auth en cada componente.

export const activeTenantId = derived(auth, $a => $a.tenantId);

// Para cuando se implemente multi-tenant switching (admin de plataforma):
// export const selectedTenantId = writable<string | null>(null);
