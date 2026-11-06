// Accounts and profiles. The profile list is static sample data until the Rust
// engine owns it; accounts and the current selection persist in localStorage.

export type AccountType = 'microsoft' | 'elyby' | 'offline';
export interface Account {
  id: string;
  name: string;
  type: AccountType;
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
  { id: 'xt', name: 'XT Optimized', kind: 'xt', loader: 'fabric', mc: '26.3' },
  { id: 'xt-plus', name: 'XT Optimized+', kind: 'xt-plus', loader: 'fabric', mc: '26.3' },
  { id: 'release', name: 'Release 26.3', kind: 'custom', loader: 'vanilla', mc: '26.3' },
  { id: 'fabric', name: 'Fabric 26.3', kind: 'custom', loader: 'fabric', mc: '26.3' },
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
  return `${LOADER_LABEL[p.loader]} · ${p.mc}`;
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

export function removeAccount(id: string) {
  const i = data.accounts.findIndex((a) => a.id === id);
  if (i < 0) return;
  data.accounts.splice(i, 1);
  if (data.accountId === id) data.accountId = data.accounts[0]?.id ?? '';
}
