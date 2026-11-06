// Accounts and profiles. The profile list is static sample data until the Rust
// engine owns it; accounts and the current selection persist in localStorage.

import { versions } from './versions.svelte';

export type AccountType = 'microsoft' | 'elyby' | 'offline';

export interface Account {
  id: string;
  name: string;
  type: AccountType;
  /** Minecraft profile uuid (microsoft / elyby) */
  uuid?: string;
}

export type Loader = 'vanilla' | 'fabric' | 'quilt' | 'forge' | 'neoforge';
export interface Profile {
  id: string;
  name: string;
  kind: 'xt' | 'xt-plus' | 'custom';
  loader: Loader;
  mc: string;
}

export const ACCOUNT_LABEL: Record<AccountType, string> = {
  microsoft: 'Microsoft',
  elyby: 'Ely.by',
  offline: 'Offline',
};

export const LOADER_LABEL: Record<Loader, string> = {
  vanilla: 'Vanilla',
  fabric: 'Fabric',
  quilt: 'Quilt',
  forge: 'Forge',
  neoforge: 'NeoForge',
};

export const PROFILES: Profile[] = [
  { id: 'xt', name: 'XT Optimized', kind: 'xt', loader: 'fabric', mc: 'latest' },
  { id: 'xt-plus', name: 'XT Optimized+', kind: 'xt-plus', loader: 'fabric', mc: 'latest' },
  { id: 'release', name: 'Vanilla Release', kind: 'custom', loader: 'vanilla', mc: 'latest' },
  { id: 'fabric', name: 'Fabric', kind: 'custom', loader: 'fabric', mc: 'latest' },
];

interface Persisted {
  accounts: Account[];
  accountId: string;
  profileId: string;
}

const KEY = 'xt.data.v1';

function load(): Persisted {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) {
      const p = JSON.parse(raw) as Persisted;
      return {
        accounts: Array.isArray(p.accounts) ? p.accounts : [],
        accountId: p.accountId ?? '',
        profileId: PROFILES.some((x) => x.id === p.profileId) ? p.profileId : 'xt',
      };
    }
  } catch {
    /* ignore */
  }
  return { accounts: [], accountId: '', profileId: 'xt' };
}

export const data: Persisted = $state(load());

$effect.root(() => {
  $effect(() => {
    const json = JSON.stringify(data);
    try {
      localStorage.setItem(KEY, json);
    } catch {
      /* ignore */
    }
  });
});

export const currentAccount = () => data.accounts.find((a) => a.id === data.accountId) ?? null;
export const currentProfile = () => PROFILES.find((p) => p.id === data.profileId) ?? PROFILES[0];

export function profileSubtitle(p: Profile) {
  const mc = p.mc === 'latest' ? versions.latestRelease || 'Latest release' : p.mc;
  return `${LOADER_LABEL[p.loader]} · ${mc}`;
}

export const OFFLINE_NAME = /^[A-Za-z0-9_]{3,16}$/;

/** Returns an error message, or null when the account was added. */
export function addOfflineAccount(name: string): string | null {
  const n = name.trim();
  if (!OFFLINE_NAME.test(n)) return 'Use 3-16 letters, numbers or underscores.';
  if (data.accounts.some((a) => a.type === 'offline' && a.name.toLowerCase() === n.toLowerCase())) {
    return 'That offline account already exists.';
  }
  const acc: Account = { id: crypto.randomUUID(), name: n, type: 'offline' };
  data.accounts.push(acc);
  data.accountId = acc.id;
  return null;
}

/** Add (or update) a signed-in account and make it the current one. */
export function addRemoteAccount(acc: Account) {
  const i = data.accounts.findIndex((a) => a.id === acc.id);
  if (i >= 0) data.accounts[i] = acc;
  else data.accounts.push(acc);
  data.accountId = acc.id;
}

export function removeAccount(id: string) {
  const i = data.accounts.findIndex((a) => a.id === id);
  if (i < 0) return;
  data.accounts.splice(i, 1);
  if (data.accountId === id) data.accountId = data.accounts[0]?.id ?? '';
}
