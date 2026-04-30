import { browser }  from '$app/environment';
import { get }       from 'svelte/store';
import { auth }      from '$lib/stores/auth';
import { redirect }  from '@sveltejs/kit';

export const prerender = false;
export const ssr       = false;

export function load({ url }: { url: URL }) {
  if (!browser) return {};
  const { token } = get(auth);
  const isPublic   = url.pathname === '/login';
  if (!token && !isPublic) throw redirect(302, '/login');
  if (token  &&  isPublic) throw redirect(302, '/dashboard');
  return {};
}
