<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import Panel from './Panel.svelte';
  import { ACCOUNT_LABEL, addOfflineAccount, data, removeAccount } from '../lib/data.svelte';
  import { cancelMicrosoft, elyLogin, forgetSecrets, initAuth, microsoftLogin, msFlow } from '../lib/auth.svelte';
  import { settings } from '../lib/settings.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { inTauri } from '../lib/engine.svelte';

  type Mode = 'none' | 'microsoft' | 'elyby' | 'offline';
  let mode = $state<Mode>('none');

  // offline
  let name = $state('');
  let offlineError = $state('');

  // ely.by
  let elyUser = $state('');
  let elyPass = $state('');
  let elyTotp = $state('');
  let elyNeedsCode = $state(false);
  let elyBusy = $state(false);
  let elyError = $state('');

  onMount(() => void initAuth());

  const toggle = (m: Mode) => (mode = mode === m ? 'none' : m);
  const needsClientId = $derived(!msFlow.hasBuiltinClient && !settings.msClientId.trim());

  function submitOffline(e: Event) {
    e.preventDefault();
    const err = addOfflineAccount(name);
    if (err) {
      offlineError = err;
      return;
    }
    name = '';
    offlineError = '';
    mode = 'none';
  }

  async function submitEly(e: Event) {
    e.preventDefault();
    elyBusy = true;
    elyError = '';
    const r = await elyLogin(elyUser, elyPass, elyNeedsCode ? elyTotp : undefined);
    elyBusy = false;
    if (r.status === 'done') {
      elyUser = elyPass = elyTotp = '';
      elyNeedsCode = false;
      mode = 'none';
    } else if (r.status === 'twoFactor') {
      elyNeedsCode = true;
    } else {
      elyError = r.message;
    }
  }

  async function signInMicrosoft() {
    if (await microsoftLogin()) mode = 'none';
  }

  async function remove(id: string) {
    await forgetSecrets(id);
    removeAccount(id);
  }

  const open = (url: string) => (inTauri ? openUrl(url) : window.open(url, '_blank'));
</script>

<Panel title="Accounts">
  {#if data.accounts.length}
    <ul class="list">
      {#each data.accounts as a (a.id)}
        <li class:active={a.id === data.accountId}>
          <button class="row" onclick={() => (data.accountId = a.id)}>
            <m3e-avatar class="avatar">{a.name.slice(0, 1).toUpperCase()}</m3e-avatar>
            <span class="text">
              <span class="title">{a.name}</span>
              <span class="sub muted">{ACCOUNT_LABEL[a.type]}</span>
            </span>
            {#if a.id === data.accountId}<span class="check"><Icon name="check" size={20} /></span>{/if}
          </button>
          <m3e-icon-button aria-label="Remove {a.name}" onclick={() => remove(a.id)}>
            <Icon name="trash" size={22} />
          </m3e-icon-button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="empty muted">No accounts yet. Add one to start playing.</p>
  {/if}

  <h3>Add an account</h3>
  <div class="options">
    <button class="opt" class:on={mode === 'microsoft'} onclick={() => toggle('microsoft')}>
      <Icon name="person" /> <span>Microsoft</span>
    </button>
    <button class="opt" class:on={mode === 'elyby'} onclick={() => toggle('elyby')}>
      <Icon name="person" /> <span>Ely.by</span>
    </button>
    <button class="opt" class:on={mode === 'offline'} onclick={() => toggle('offline')}>
      <Icon name="personAdd" /> <span>Offline</span>
    </button>
  </div>

  {#if mode === 'microsoft'}
    <div class="box">
      {#if msFlow.waiting}
        <p>Enter this code on the Microsoft page that just opened:</p>
        {#if msFlow.code}
          <div class="code" aria-live="polite">{msFlow.code}</div>
          <p class="hint muted">Page didn't open? Go to {msFlow.url}</p>
        {:else}
          <p class="muted">Requesting a sign-in code…</p>
        {/if}
        <div class="actions">
          {#if msFlow.url}<m3e-button variant="tonal" onclick={() => open(msFlow.url)}>Open page</m3e-button>{/if}
          <m3e-button variant="text" onclick={cancelMicrosoft}>Cancel</m3e-button>
        </div>
      {:else}
        {#if needsClientId}
          <p>
            Microsoft sign-in needs an Azure application (client) id. Register a free app in Microsoft Entra
            (account type: personal Microsoft accounts, platform: mobile and desktop, public client flows on) and
            paste its Application ID.
          </p>
          <m3e-form-field variant="outlined" class="field">
            <label slot="label" for="ms-client">Application (client) ID</label>
            <input id="ms-client" bind:value={settings.msClientId} spellcheck="false" autocomplete="off" />
          </m3e-form-field>
          <p class="hint muted">
            Mojang must also approve the id for Minecraft services. Until then sign-in can fail with a
            “not allowed” message.
            <button class="link" onclick={() => open('https://aka.ms/mce-reviewappid')}>Request approval</button>
          </p>
        {:else}
          <p class="muted">Sign in with the Microsoft account that owns Minecraft: Java Edition.</p>
        {/if}
        <div class="actions">
          <m3e-button variant="filled" disabled={needsClientId} onclick={signInMicrosoft}>Sign in with Microsoft</m3e-button>
        </div>
      {/if}
      {#if msFlow.error}<p class="error" role="alert">{msFlow.error}</p>{/if}
    </div>
  {/if}

  {#if mode === 'elyby'}
    <form class="box" onsubmit={submitEly}>
      <m3e-form-field variant="outlined" class="field">
        <label slot="label" for="ely-user">Username or email</label>
        <input id="ely-user" bind:value={elyUser} autocomplete="username" spellcheck="false" />
      </m3e-form-field>
      <m3e-form-field variant="outlined" class="field">
        <label slot="label" for="ely-pass">Password</label>
        <input id="ely-pass" type="password" bind:value={elyPass} autocomplete="current-password" />
      </m3e-form-field>
      {#if elyNeedsCode}
        <m3e-form-field variant="outlined" class="field">
          <label slot="label" for="ely-totp">Two-factor code</label>
          <input id="ely-totp" bind:value={elyTotp} inputmode="numeric" autocomplete="one-time-code" maxlength="8" />
        </m3e-form-field>
      {/if}
      {#if elyError}<p class="error" role="alert">{elyError}</p>{/if}
      <div class="actions">
        <m3e-button type="submit" variant="filled" disabled={elyBusy || !elyUser || !elyPass}>
          {elyBusy ? 'Signing in…' : 'Sign in'}
        </m3e-button>
      </div>
      <p class="hint muted">Your password is sent only to authserver.ely.by and is never stored.</p>
    </form>
  {/if}

  {#if mode === 'offline'}
    <form class="box" onsubmit={submitOffline}>
      <div class="inline">
        <m3e-form-field variant="outlined" class="field">
          <label slot="label" for="offline-name">Username</label>
          <input id="offline-name" bind:value={name} maxlength="16" autocomplete="off" spellcheck="false" />
        </m3e-form-field>
        <m3e-button type="submit" variant="filled">Add</m3e-button>
      </div>
      {#if offlineError}<p class="error" role="alert">{offlineError}</p>{/if}
      <p class="hint muted">Offline accounts work without signing in, but can't join online-mode servers.</p>
    </form>
  {/if}
</Panel>

<style>
  .list {
    list-style: none;
    margin: 4px 0 8px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 4px;
    border-radius: 20px;
    padding-right: 4px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 55%, transparent);
  }
  li.active {
    background: color-mix(in srgb, var(--md-sys-color-primary-container, #8f2410) 70%, transparent);
  }
  .row {
    all: unset;
    box-sizing: border-box;
    flex: 1;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    cursor: pointer;
    min-width: 0;
  }
  .row:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
    border-radius: 20px;
  }
  .avatar {
    --m3e-avatar-size: 40px;
  }
  .text {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }
  .title {
    font-weight: 600;
  }
  .sub {
    font-size: 12.5px;
  }
  .check {
    display: grid;
    place-items: center;
  }
  .empty {
    margin: 8px 0 12px;
  }
  h3 {
    margin: 18px 0 8px;
    font-size: 13px;
    font-weight: 650;
    letter-spacing: 0.02em;
    color: var(--md-sys-color-primary, #c23a24);
  }
  .options {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .opt {
    all: unset;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 14px 8px;
    border-radius: 20px;
    cursor: pointer;
    font-weight: 600;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 55%, transparent);
  }
  .opt:hover {
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 90%, transparent);
  }
  .opt.on {
    background: var(--md-sys-color-secondary-container, #5d3f3a);
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
  }
  .opt:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
  }

  .box {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 14px;
    padding: 16px;
    border-radius: 20px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-high, #2d2929) 60%, transparent);
  }
  .box p {
    margin: 0;
    font-size: 13.5px;
    line-height: 1.5;
  }
  .inline {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .field {
    flex: 1;
    width: 100%;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .code {
    font-size: 34px;
    font-weight: 700;
    letter-spacing: 0.18em;
    padding: 6px 0;
    user-select: text;
    color: var(--md-sys-color-primary, #c23a24);
  }
  .error {
    color: var(--md-sys-color-error, #ffb4ab);
  }
  .hint {
    font-size: 12.5px;
  }
  .link {
    all: unset;
    cursor: pointer;
    text-decoration: underline;
    color: var(--md-sys-color-primary, #c23a24);
  }
</style>
