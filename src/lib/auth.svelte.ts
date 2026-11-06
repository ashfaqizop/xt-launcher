// Account sign-in flows (Microsoft device code, Ely.by) on top of the Rust engine.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';
import { addRemoteAccount, type Account } from './data.svelte';
import { inTauri } from './engine.svelte';
import { settings } from './settings.svelte';

interface AccountResult {
  id: string;
  name: string;
  uuid: string;
  kind: 'microsoft' | 'elyby';
}

export const msFlow = $state({
  waiting: false,
  code: '',
  url: '',
  error: '',
  hasBuiltinClient: false,
});

export async function initAuth() {
  if (!inTauri) return;
  try {
    msFlow.hasBuiltinClient = await invoke<boolean>('ms_has_builtin_client');
  } catch {
    /* ignore */
  }
}

function toAccount(r: AccountResult): Account {
  return { id: r.id, name: r.name, uuid: r.uuid, type: r.kind };
}

export async function microsoftLogin(): Promise<boolean> {
  if (!inTauri) {
    msFlow.error = 'Microsoft sign-in works in the desktop app.';
    return false;
  }
  msFlow.error = '';
  msFlow.waiting = true;
  msFlow.code = '';

  const unlisten = await listen<{ userCode: string; verificationUri: string }>(
    'auth-device-code',
    (e) => {
      msFlow.code = e.payload.userCode;
      msFlow.url = e.payload.verificationUri;
      void openUrl(e.payload.verificationUri).catch(() => {});
    },
  );

  try {
    const result = await invoke<AccountResult>('ms_login', {
      clientId: settings.msClientId || null,
    });
    addRemoteAccount(toAccount(result));
    return true;
  } catch (err) {
    msFlow.error = String(err);
    return false;
  } finally {
    unlisten();
    msFlow.waiting = false;
    msFlow.code = '';
  }
}

export async function cancelMicrosoft() {
  try {
    await invoke('ms_cancel');
  } catch {
    /* ignore */
  }
}

export type ElyResult = { status: 'done' } | { status: 'twoFactor' } | { status: 'error'; message: string };

export async function elyLogin(username: string, password: string, totp?: string): Promise<ElyResult> {
  if (!inTauri) return { status: 'error', message: 'Ely.by sign-in works in the desktop app.' };
  try {
    const out = await invoke<{ status: 'done'; account: AccountResult } | { status: 'twoFactor' }>(
      'ely_login',
      { username, password, totp: totp || null },
    );
    if (out.status === 'twoFactor') return { status: 'twoFactor' };
    addRemoteAccount(toAccount(out.account));
    return { status: 'done' };
  } catch (err) {
    return { status: 'error', message: String(err) };
  }
}

export async function forgetSecrets(id: string) {
  if (!inTauri) return;
  try {
    await invoke('forget_account', { id });
  } catch {
    /* ignore */
  }
}
