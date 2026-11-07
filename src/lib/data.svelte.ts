// Accounts and profiles, persisted in localStorage. There are no built-in profiles:
// the user creates them, with XT Optimized / XT Optimized+ offered as recommended templates.

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
  /** A Minecraft version id, or "latest" for the newest release */
  mc: string;
  /** Per-profile memory override in MB (null = use the global setting) */
  ramMb?: number | null;
  /** Extra JVM arguments, space separated */
  jvmArgs?: string;
  /** Use the ZGC garbage collector (always on for XT profiles) */
  zgc?: boolean;
  /** Total time played, in ms */
  playMs?: number;
  /** When it was last played (epoch ms) */
  lastPlayed?: number;
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

export type ProfileKind = Profile['kind'];

/** A starting point offered on the New profile page. */
export interface Template {
  kind: ProfileKind;
  name: string;
  summary: string;
  mods: string;
  recommended: boolean;
}

export const TEMPLATES: Template[] = [
  {
    kind: 'xt',
    name: 'XT Optimized',
    summary: 'Fabric with hand-picked performance mods and ZGC. The fastest way to play.',
    mods: 'Sodium, Lithium, FerriteCore, Entity Culling, ImmediatelyFast, ModernFix, Dynamic FPS',
    recommended: true,
  },
  {
    kind: 'xt-plus',
    name: 'XT Optimized+',
    summary: 'Everything in XT Optimized plus quality-of-life mods.',
    mods: "Mod Menu, Xaero's World Map and Minimap, AppleSkin, Zoomify, Shulker Box Tooltip, Gamma Utilities",
    recommended: true,
  },
  {
    kind: 'custom',
    name: 'Custom',
    summary: 'Pick any loader and version: Vanilla, Fabric, Quilt, Forge or NeoForge.',
    mods: '',
    recommended: false,
  },
];

/** XT profiles always run Fabric with the curated mods and ZGC. */
export const isXt = (p: Pick<Profile, 'kind'>) => p.kind !== 'custom';

interface Persisted {
  accounts: Account[];
  accountId: string;
  profileId: string;
  /** Profiles the user created */
  profiles: Profile[];
}

const KEY = 'xt.data.v1';

function load(): Persisted {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) {
      const p = JSON.parse(raw) as Partial<Persisted>;
      const profiles = Array.isArray(p.profiles) ? p.profiles : [];
      return {
        accounts: Array.isArray(p.accounts) ? p.accounts : [],
        accountId: p.accountId ?? '',
        profileId: profiles.some((x) => x.id === p.profileId) ? (p.profileId as string) : (profiles[0]?.id ?? ''),
        profiles,
      };
    }
  } catch {
    /* ignore */
  }
  return { accounts: [], accountId: '', profileId: '', profiles: [] };
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
export const currentProfile = (): Profile | null => data.profiles.find((p) => p.id === data.profileId) ?? null;

/** A new, unsaved profile based on a template. */
export function profileFromTemplate(kind: ProfileKind): Profile {
  const t = TEMPLATES.find((x) => x.kind === kind) ?? TEMPLATES[2];
  const taken = new Set(data.profiles.map((p) => p.name));
  let name = kind === 'custom' ? '' : t.name;
  for (let n = 2; name && taken.has(name); n++) name = `${t.name} ${n}`;
  return {
    id: crypto.randomUUID(),
    name,
    kind,
    loader: kind === 'custom' ? 'vanilla' : 'fabric',
    mc: 'latest',
    ramMb: null,
    jvmArgs: '',
    zgc: kind !== 'custom',
  };
}

/** Add or update a profile (XT profiles are always Fabric). */
export function saveProfile(p: Profile) {
  const next: Profile = isXt(p) ? { ...p, loader: 'fabric', zgc: true } : { ...p };
  const i = data.profiles.findIndex((x) => x.id === p.id);
  if (i >= 0) data.profiles[i] = next;
  else data.profiles.push(next);
}

export function duplicateProfile(id: string): string | null {
  const src = data.profiles.find((p) => p.id === id);
  if (!src) return null;
  const copy: Profile = { ...src, id: crypto.randomUUID(), name: `${src.name} copy` };
  data.profiles.push(copy);
  return copy.id;
}

/** Add a finished session's play time to its profile. */
export function recordPlaytime(id: string, ms: number) {
  const p = data.profiles.find((x) => x.id === id);
  if (!p) return;
  p.playMs = (p.playMs ?? 0) + ms;
  p.lastPlayed = Date.now();
}

/** "3h 12m", "45m", "under a minute" */
export function formatPlaytime(ms: number): string {
  const minutes = Math.floor(ms / 60000);
  if (minutes < 1) return 'under a minute';
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return h ? h + 'h ' + m + 'm' : m + 'm';
}

export function deleteProfile(id: string) {
  const i = data.profiles.findIndex((p) => p.id === id);
  if (i < 0) return;
  data.profiles.splice(i, 1);
  if (data.profileId === id) data.profileId = data.profiles[0]?.id ?? '';
}

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

/** Rename an offline account. Returns an error message, or null on success. */
export function renameOfflineAccount(id: string, name: string): string | null {
  const n = name.trim();
  const acc = data.accounts.find((a) => a.id === id);
  if (!acc || acc.type !== 'offline') return 'Only offline accounts can be renamed.';
  if (!OFFLINE_NAME.test(n)) return 'Use 3-16 letters, numbers or underscores.';
  if (data.accounts.some((a) => a.id !== id && a.type === 'offline' && a.name.toLowerCase() === n.toLowerCase())) {
    return 'You already have this offline account.';
  }
  acc.name = n;
  return null;
}

export function removeAccount(id: string) {
  const i = data.accounts.findIndex((a) => a.id === id);
  if (i < 0) return;
  data.accounts.splice(i, 1);
  if (data.accountId === id) data.accountId = data.accounts[0]?.id ?? '';
}
