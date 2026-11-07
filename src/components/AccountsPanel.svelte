<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import Head from '../lib/Head.svelte';
  import Panel from './Panel.svelte';
  import { ACCOUNT_LABEL, OFFLINE_NAME, addOfflineAccount, data, removeAccount, type AccountType } from '../lib/data.svelte';
  import { cancelAuth, elyFlow, elyLogin, forgetSecrets, initAuth } from '../lib/auth.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { inTauri } from '../lib/engine.svelte';

  type Mode = 'elyby' | 'offline' | null;
  let mode = $state<Mode>(null);
  let confirmRemove = $state<string | null>(null);

  // Offline
  let name = $state('');
  let nameInput = $state<HTMLInputElement>();
  let touched = $state(false);

  onMount(() => {
    void initAuth();
    // Nothing yet: open the recommended way straight away
    if (data.accounts.length === 0) mode = 'elyby';
  });

  async function pick(m: Exclude<Mode, null>) {
    mode = mode === m ? null : m;
    if (mode === 'offline') {
      await tick();
      nameInput?.focus();
    }
  }

  // Live validation for the offline name
  const trimmed = $derived(name.trim());
  const duplicate = $derived(
    data.accounts.some((a) => a.type === 'offline' && a.name.toLowerCase() === trimmed.toLowerCase()),
  );
  const nameProblem = $derived.by(() => {
    if (!trimmed) return '';
    if (trimmed.length < 3) return 'At least 3 characters';
    if (!OFFLINE_NAME.test(trimmed)) return 'Only letters, numbers and _';
    if (duplicate) return 'You already have this offline account';
    return '';
  });
  const nameOk = $derived(!!trimmed && !nameProblem);

  // Preview the skin for the typed name, a moment after typing stops
  let previewName = $state('');
  $effect(() => {
    const n = trimmed;
    const t = setTimeout(() => (previewName = n), 350);
    return () => clearTimeout(t);
  });

  function addOffline(e: Event) {
    e.preventDefault();
    touched = true;
    if (!nameOk) return;
    if (addOfflineAccount(trimmed) === null) {
      name = '';
      touched = false;
      mode = null;
    }
  }

  async function signInElyby() {
    if (await elyLogin()) mode = null;
  }

  async function remove(id: string) {
    if (confirmRemove !== id) {
      confirmRemove = id;
      return;
    }
    confirmRemove = null;
    await forgetSecrets(id);
    removeAccount(id);
  }

  const open = (url: string) => (inTauri ? openUrl(url) : window.open(url, '_blank'));

  const KIND_CLASS: Record<AccountType, string> = { elyby: 'ely', offline: 'off', microsoft: 'ms' };
</script>

<Panel title="Accounts">
  <!-- Your accounts -->
  {#if data.accounts.length}
    <ul class="list">
      {#each data.accounts as a (a.id)}
        {@const active = a.id === data.accountId}
        <li class:active>
          <button class="row" onclick={() => (data.accountId = a.id)} aria-pressed={active}>
            <Head name={a.name} size={42} />
            <span class="text">
              <span class="title">{a.name}</span>
              <span class="meta">
                <span class="kind {KIND_CLASS[a.type]}">{ACCOUNT_LABEL[a.type]}</span>
                {#if a.type === 'offline'}<span class="muted">Single-player and offline servers</span>{/if}
              </span>
            </span>
            {#if active}
              <span class="inuse"><Icon name="check" size={16} />In use</span>
            {:else}
              <span class="use muted">Use</span>
            {/if}
          </button>
          {#if confirmRemove === a.id}
            <m3e-button class="danger" variant="filled" size="small" onclick={() => remove(a.id)}>Remove</m3e-button>
            <m3e-icon-button size="small" aria-label="Keep {a.name}" onclick={() => (confirmRemove = null)}>
              <Icon name="close" size={18} />
            </m3e-icon-button>
          {:else}
            <m3e-icon-button aria-label="Remove {a.name}" onclick={() => remove(a.id)}>
              <Icon name="trash" size={20} />
            </m3e-icon-button>
          {/if}
        </li>
      {/each}
    </ul>
  {:else}
    <div class="empty">
      <span class="empty-icon"><Icon name="person" size={26} /></span>
      <div>
        <div class="title">No accounts yet</div>
        <div class="muted small">Sign in with Ely.by for skins and online play, or play offline with just a name.</div>
      </div>
    </div>
  {/if}

  <!-- Add -->
  <h3>Add an account</h3>
  <div class="providers">
    <button class="provider" class:on={mode === 'elyby'} onclick={() => pick('elyby')} aria-expanded={mode === 'elyby'}>
      <span class="logo ely">E</span>
      <span class="p-text">
        <span class="p-name">Ely.by <span class="rec">Recommended</span></span>
        <span class="p-desc muted">Free account with skins and capes. Works on servers that support Ely.by.</span>
      </span>
    </button>
      {#if mode === 'elyby'}
        <div class="flow">
          <ol class="steps">
            <li class:done={elyFlow.waiting} class:now={!elyFlow.waiting}><span>1</span>Open Ely.by in your browser</li>
            <li class:now={elyFlow.waiting}><span>2</span>Log in and press Authorize</li>
            <li><span>3</span>Come back here, you're connected</li>
          </ol>

          {#if elyFlow.waiting}
            <div class="waiting">
              <m3e-circular-progress-indicator variant="wavy" indeterminate class="spin"></m3e-circular-progress-indicator>
              <span>Waiting for you to authorize XT Launcher…</span>
            </div>
            <div class="actions">
              {#if elyFlow.url}<m3e-button variant="tonal" onclick={() => open(elyFlow.url)}>Open the page again</m3e-button>{/if}
              <m3e-button variant="text" onclick={cancelAuth}>Cancel</m3e-button>
            </div>
          {:else}
            <div class="actions">
              <m3e-button variant="filled" onclick={signInElyby}>
                <span slot="icon"><Icon name="open" size={18} /></span>Continue in browser
              </m3e-button>
              <m3e-button variant="text" onclick={() => open('https://account.ely.by/register')}>Create an Ely.by account</m3e-button>
            </div>
            <p class="hint muted">XT Launcher never sees your password. You can revoke access any time on ely.by.</p>
          {/if}
          {#if elyFlow.error}<p class="error" role="alert">{elyFlow.error}</p>{/if}
        </div>
      {/if}

    <button class="provider" class:on={mode === 'offline'} onclick={() => pick('offline')} aria-expanded={mode === 'offline'}>
      <span class="logo off"><Icon name="person" size={22} /></span>
      <span class="p-text">
        <span class="p-name">Offline</span>
        <span class="p-desc muted">Just a name. Single-player, LAN and offline-mode servers.</span>
      </span>
    </button>
      {#if mode === 'offline'}
        <form class="flow offline" onsubmit={addOffline}>
          <Head name={previewName} size={56} />
          <div class="grow">
            <m3e-form-field variant="outlined" class="field" error={touched && !!nameProblem}>
              <label slot="label" for="offline-name">Player name</label>
              <input
                id="offline-name"
                bind:this={nameInput}
                bind:value={name}
                maxlength="16"
                autocomplete="off"
                spellcheck="false"
                onblur={() => (touched = !!name)}
              />
              <span slot="hint">{nameProblem || 'Skins show up if this name has an Ely.by skin'}</span>
              <span slot="suffix-text" class="count">{trimmed.length}/16</span>
            </m3e-form-field>
          </div>
          <m3e-button type="submit" variant="filled" disabled={!nameOk}>Add</m3e-button>
        </form>
      {/if}

    <button class="provider" disabled aria-disabled="true">
      <span class="logo ms" aria-hidden="true"><svg viewBox="0 0 20 20" width="20" height="20"><rect x="0" y="0" width="9" height="9" fill="#f25022"/><rect x="11" y="0" width="9" height="9" fill="#7fba00"/><rect x="0" y="11" width="9" height="9" fill="#00a4ef"/><rect x="11" y="11" width="9" height="9" fill="#ffb900"/></svg></span>
      <span class="p-text">
        <span class="p-name">Microsoft <span class="soon">Soon</span></span>
        <span class="p-desc muted">Your Minecraft: Java Edition purchase.</span>
      </span>
    </button>
  </div>

</Panel>

<style>
  .list {
    list-style: none;
    margin: 2px 0 6px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 4px;
    padding-right: 6px;
    border-radius: 20px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 50%, transparent);
    transition: background 140ms;
  }
  li.active {
    background: color-mix(in srgb, var(--md-sys-color-primary-container, #8f2410) 62%, transparent);
  }
  .row {
    all: unset;
    box-sizing: border-box;
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 8px 10px 10px;
    cursor: pointer;
  }
  .row:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
    border-radius: 20px;
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .title {
    font-size: 15px;
    font-weight: 650;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    min-width: 0;
  }
  .meta .muted {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .kind {
    flex: none;
    padding: 1px 8px;
    border-radius: 999px;
    font-weight: 650;
    font-size: 11px;
  }
  .kind.ely {
    background: #1f7a5a;
    color: #e6fff4;
  }
  .kind.off {
    background: var(--md-sys-color-surface-container-highest, #383434);
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .kind.ms {
    background: #2b5797;
    color: #fff;
  }
  .inuse {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    font-weight: 650;
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--md-sys-color-primary, #ffb4a6);
    color: var(--md-sys-color-on-primary, #5f150a);
  }
  .use {
    font-size: 12.5px;
    font-weight: 600;
    padding: 0 6px;
    opacity: 0;
    transition: opacity 120ms;
  }
  li:hover .use {
    opacity: 1;
  }
  .danger {
    --m3e-button-container-color: var(--md-sys-color-error, #ffb4ab);
    --m3e-button-label-text-color: var(--md-sys-color-on-error, #690005);
  }

  .empty {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px;
    border-radius: 20px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 40%, transparent);
  }
  .empty-icon {
    width: 44px;
    height: 44px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 14px;
    background: var(--md-sys-color-secondary-container, #5d3f3a);
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
  }
  .small {
    font-size: 13px;
    line-height: 1.45;
  }

  h3 {
    margin: 18px 4px 8px;
    font-size: 13px;
    font-weight: 650;
    letter-spacing: 0.02em;
    color: var(--md-sys-color-primary, #c23a24);
  }

  .providers {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .provider {
    all: unset;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 14px 12px 12px;
    border-radius: 18px;
    cursor: pointer;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 45%, transparent);
    outline: 1px solid transparent;
    transition:
      background 120ms,
      outline-color 120ms;
  }
  .provider:hover:not(:disabled) {
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 85%, transparent);
  }
  .provider.on {
    outline-color: color-mix(in srgb, var(--md-sys-color-primary, #c23a24) 60%, transparent);
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 85%, transparent);
  }
  .provider:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
  }
  .provider:disabled {
    cursor: default;
    opacity: 0.5;
  }
  .logo {
    width: 40px;
    height: 40px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 12px;
    font-weight: 800;
    font-size: 20px;
  }
  .logo.ely {
    background: #1f7a5a;
    color: #fff;
  }
  .logo.off {
    background: var(--md-sys-color-secondary-container, #5d3f3a);
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
  }
  .logo.ms {
    background: var(--md-sys-color-surface-container-highest, #383434);
  }
  .provider:disabled .logo.ms svg {
    filter: grayscale(1);
  }
  .p-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .p-name {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14.5px;
    font-weight: 650;
  }
  .p-desc {
    font-size: 12.5px;
    line-height: 1.4;
  }
  .rec,
  .soon {
    font-size: 10.5px;
    font-weight: 650;
    padding: 2px 8px;
    border-radius: 999px;
  }
  .rec {
    background: var(--md-sys-color-tertiary-container, #5b4a1f);
    color: var(--md-sys-color-on-tertiary-container, #ffe08c);
  }
  .soon {
    background: var(--md-sys-color-surface-container-highest, #383434);
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }

  .flow {
    margin: 2px 0 6px;
    padding: 16px;
    border-radius: 20px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-high, #2d2929) 70%, transparent);
    display: flex;
    flex-direction: column;
    gap: 12px;
    animation: open 220ms cubic-bezier(0.2, 0, 0, 1);
  }
  .flow.offline {
    flex-direction: row;
    align-items: center;
    gap: 14px;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .field {
    width: 100%;
  }
  .count {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    opacity: 0.7;
  }

  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .steps li {
    all: unset;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13.5px;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .steps li span {
    width: 22px;
    height: 22px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    font-size: 12px;
    font-weight: 700;
    background: var(--md-sys-color-surface-container-highest, #383434);
  }
  .steps li.now {
    color: var(--md-sys-color-on-surface, #ece0dd);
    font-weight: 600;
  }
  .steps li.now span {
    background: var(--md-sys-color-primary, #ffb4a6);
    color: var(--md-sys-color-on-primary, #5f150a);
  }
  .steps li.done span {
    background: #1f7a5a;
    color: #fff;
  }

  .waiting {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 13.5px;
  }
  .spin {
    --m3e-circular-wavy-progress-indicator-diameter: 28px;
    --m3e-circular-progress-indicator-thickness: 3px;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .hint {
    margin: 0;
    font-size: 12.5px;
  }
  .error {
    margin: 0;
    font-size: 13px;
    color: var(--md-sys-color-error, #ffb4ab);
  }
  @keyframes open {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
</style>
