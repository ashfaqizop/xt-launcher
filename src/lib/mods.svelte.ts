// The mod manager's data: what is in a profile's mods folder, who made it (Modrinth), and
// switching mods on and off.

import { invoke } from '@tauri-apps/api/core';

export interface ModEntry {
  /** The file on disk; ends in ".disabled" when the mod is switched off */
  file: string;
  enabled: boolean;
  name: string;
  version: string;
  description: string;
  author: string;
  modId: string;
  loader: string;
  icon: string | null;
  size: number;
  modified: number;
  /** xt: kept up to date by the XT tier; modrinth: installed from the browser; manual: added by hand */
  source: 'xt' | 'modrinth' | 'manual';
  projectId: string | null;
  sha1: string;
}

export interface Identified {
  projectId: string;
  slug: string;
  title: string;
  iconUrl: string | null;
  version: string;
}

const KEY = 'xt.modident.v1';

function loadIdentity(): Record<string, Identified | null> {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return JSON.parse(raw);
  } catch {
    /* ignore */
  }
  return {};
}

export const modsState = $state({
  profileId: '',
  list: [] as ModEntry[],
  loading: false,
  error: '',
  /** Modrinth identity by jar SHA-1 (null = checked, not on Modrinth) */
  identity: loadIdentity() as Record<string, Identified | null>,
});

let token = 0;

function saveIdentity() {
  try {
    localStorage.setItem(KEY, JSON.stringify(modsState.identity));
  } catch {
    /* ignore */
  }
}

/** Look up jars we haven't identified yet. Quiet when offline. */
async function identifyMissing(mine: number) {
  const missing = modsState.list.map((m) => m.sha1).filter((h) => h && !(h in modsState.identity));
  if (!missing.length) return;
  try {
    const found = await invoke<Record<string, Identified>>('modrinth_identify', { hashes: [...new Set(missing)] });
    if (mine !== token) return;
    for (const hash of new Set(missing)) modsState.identity[hash] = found[hash] ?? null;
    saveIdentity();
  } catch {
    /* offline: try again next time */
  }
}

export async function loadMods(profileId: string, quiet = false) {
  const mine = ++token;
  if (modsState.profileId !== profileId) {
    modsState.list = [];
    modsState.profileId = profileId;
  }
  if (!quiet) modsState.loading = true;
  modsState.error = '';
  try {
    const list = await invoke<ModEntry[]>('list_mods', { profileId });
    if (mine !== token) return;
    modsState.list = list;
  } catch (err) {
    if (mine !== token) return;
    modsState.error = String(err);
  } finally {
    if (mine === token) modsState.loading = false;
  }
  if (mine === token) void identifyMissing(mine);
}

/** The Modrinth identity of one mod, if known. */
export const identityOf = (mod: ModEntry): Identified | null => modsState.identity[mod.sha1] ?? null;

/** Project ids of everything installed (to mark results as installed in the browser). */
export function installedProjectIds(): Set<string> {
  const ids = new Set<string>();
  for (const mod of modsState.list) {
    const id = mod.projectId ?? identityOf(mod)?.projectId;
    if (id) ids.add(id);
  }
  return ids;
}

export async function setModEnabled(profileId: string, mod: ModEntry, enabled: boolean) {
  const file = await invoke<string>('set_mod_enabled', { profileId, file: mod.file, enabled });
  const entry = modsState.list.find((m) => m.sha1 === mod.sha1 && m.file === mod.file);
  if (entry) {
    entry.file = file;
    entry.enabled = enabled;
  }
}

export async function removeMod(profileId: string, mod: ModEntry) {
  await invoke('remove_mod', { profileId, file: mod.file });
  modsState.list = modsState.list.filter((m) => m.file !== mod.file);
}
