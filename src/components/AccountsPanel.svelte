<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import Head from '../lib/Head.svelte';
  import SkinView from '../lib/SkinView.svelte';
  import Panel from './Panel.svelte';
  import { sortable } from '../lib/sortable';
  import {
    ACCOUNT_LABEL,
    OFFLINE_NAME,
    moveAccount,
    addOfflineAccount,
    data,
    removeAccount,
    renameOfflineAccount,
    type AccountType,
  } from '../lib/data.svelte';
  import { cancelAuth, elyFlow, elyLogin, forgetSecrets, initAuth } from '../lib/auth.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { inTauri } from '../lib/engine.svelte';
  import { showToast } from '../lib/toast.svelte';

  // Right pane: one account's details, or the add flow
  type Pane = { kind: 'account'; id: string } | { kind: 'add' };
  let pane = $state<Pane>({ kind: 'add' });
  type Method = 'elyby' | 'offline';
  let method = $state<Method>('elyby');

  onMount(() => {
    void initAuth();
    const id = data.accountId || data.accounts[0]?.id;
    pane = id ? { kind: 'account', id } : { kind: 'add' };
  });

  const selected = $derived.by(() => {
    const p = pane;
    return p.kind === 'account' ? (data.accounts.find((a) => a.id === p.id) ?? null) : null;
  });

  function show(id: string) {
    pane = { kind: 'account', id };
    confirmRemove = false;
    renaming = false;
  }
  async function startAdd(m: Method = 'elyby') {
    pane = { kind: 'add' };
    method = m;
    if (m === 'offline') {
      await tick();
      nameInput?.focus();
    }
  }

  // ---- Account details -------------------------------------------------------

  let confirmRemove = $state(false);
  let renaming = $state(false);
  let renameValue = $state('');
  let renameError = $state('');

  async function remove() {
    if (!selected) return;
    if (!confirmRemove) {
      confirmRemove = true;
      return;
    }
    const id = selected.id;
    confirmRemove = false;
    await forgetSecrets(id);
    removeAccount(id);
    const next = data.accountId || data.accounts[0]?.id;
    pane = next ? { kind: 'account', id: next } : { kind: 'add' };
  }

  function beginRename() {
    if (!selected) return;
    renameValue = selected.name;
    renameError = '';
    renaming = true;
  }
  function saveRename(e: Event) {
    e.preventDefault();
    if (!selected) return;
    const err = renameOfflineAccount(selected.id, renameValue);
    if (err) {
      renameError = err;
      return;
    }
    renaming = false;
  }

  function copyName() {
    if (!selected) return;
    void navigator.clipboard
      .writeText(selected.name)
      .then(() => showToast('Name copied'))
      .catch(() => {});
  }

  // ---- Add: offline ---------------------------------------------------------

  let name = $state('');
  let nameInput = $state<HTMLInputElement>();
  const trimmed = $derived(name.trim());
  const nameProblem = $derived.by(() => {
    if (!trimmed) return '';
    if (trimmed.length < 3) return 'At least 3 characters';
    if (!OFFLINE_NAME.test(trimmed)) return 'Only letters, numbers and _';
    if (data.accounts.some((a) => a.type === 'offline' && a.name.toLowerCase() === trimmed.toLowerCase())) {
      return 'You already have this offline account';
    }
    return '';
  });
  const nameOk = $derived(!!trimmed && !nameProblem);

  let previewName = $state('');
  $effect(() => {
    const n = trimmed;
    const t = setTimeout(() => (previewName = n), 350);
    return () => clearTimeout(t);
  });

  function addOffline(e: Event) {
    e.preventDefault();
    if (!nameOk) return;
    if (addOfflineAccount(trimmed) === null) {
      name = '';
      show(data.accountId);
    }
  }

  // ---- Add: Ely.by ----------------------------------------------------------

  async function signInElyby() {
    if (await elyLogin()) show(data.accountId);
  }

  const open = (url: string) => (inTauri ? openUrl(url) : window.open(url, '_blank'));
  const KIND_CLASS: Record<AccountType, string> = { elyby: 'ely', offline: 'off', microsoft: 'ms' };
</script>

<Panel title="Accounts" wide>
  <div class="split">
    <!-- Left: your accounts -->
    <aside class="side">
      <div class="side-title muted">Your accounts</div>
      {#if data.accounts.length}
        <ul class="list" use:sortable={{ onMove: moveAccount }}>
          {#each data.accounts as a (a.id)}
            <li>
              <button
                type="button"
                class="grip"
                data-handle
                aria-label="Reorder {a.name}. Drag it, or hold Alt and press the up or down arrow."
              >
                <Icon name="drag" size={18} />
              </button>
              <button
                class="item"
                class:sel={pane.kind === 'account' && pane.id === a.id}
                onclick={() => show(a.id)}
              >
                <Head name={a.name} size={36} />
                <span class="item-text">
                  <span class="item-name">{a.name}</span>
                  <span class="kind {KIND_CLASS[a.type]}">{ACCOUNT_LABEL[a.type]}</span>
                </span>
                {#if a.id === data.accountId}<span class="dot" title="In use"></span>{/if}
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="none muted">No accounts yet.</p>
      {/if}
      <m3e-button class="add" variant={pane.kind === 'add' ? 'filled' : 'tonal'} onclick={() => startAdd()}>
        <span slot="icon"><Icon name="add" size={20} /></span>Add account
      </m3e-button>
    </aside>

    <!-- Right: details or add flow -->
    <section class="main">
      {#if pane.kind === 'account' && selected}
        {#key selected.id}
          <div class="detail">
            <div class="hero">
              <SkinView name={selected.name} unit={6} stage />
              <div class="hero-text">
                {#if renaming}
                  <form class="rename" onsubmit={saveRename}>
                    <m3e-form-field variant="outlined" class="field" error={!!renameError}>
                      <label slot="label" for="rename">Player name</label>
                      <input id="rename" bind:value={renameValue} maxlength="16" spellcheck="false" autocomplete="off" />
                      {#if renameError}<span slot="error">{renameError}</span>{/if}
                    </m3e-form-field>
                    <div class="row-actions">
                      <m3e-button type="submit" variant="filled" size="small">Save</m3e-button>
                      <m3e-button variant="text" size="small" onclick={() => (renaming = false)}>Cancel</m3e-button>
                    </div>
                  </form>
                {:else}
                  <div class="big-name">{selected.name}</div>
                {/if}
                <span class="kind {KIND_CLASS[selected.type]}">{ACCOUNT_LABEL[selected.type]}</span>

                <dl class="facts">
                  <div>
                    <dt>Status</dt>
                    <dd>{selected.id === data.accountId ? 'In use' : 'Not in use'}</dd>
                  </div>
                  <div>
                    <dt>Plays on</dt>
                    <dd>
                      {selected.type === 'elyby'
                        ? 'Single-player, LAN and servers that support Ely.by'
                        : 'Single-player, LAN and offline-mode servers'}
                    </dd>
                  </div>
                  {#if selected.uuid}
                    <div>
                      <dt>UUID</dt>
                      <dd class="mono">{selected.uuid}</dd>
                    </div>
                  {/if}
                </dl>
              </div>
            </div>

            <div class="actions">
              {#if selected.id !== data.accountId}
                <m3e-button variant="filled" onclick={() => (data.accountId = selected!.id)}>
                  <span slot="icon"><Icon name="check" size={18} /></span>Use this account
                </m3e-button>
              {:else}
                <m3e-button variant="tonal" disabled>
                  <span slot="icon"><Icon name="check" size={18} /></span>In use
                </m3e-button>
              {/if}
              {#if selected.type === 'offline' && !renaming}
                <m3e-button variant="text" onclick={beginRename}>
                  <span slot="icon"><Icon name="edit" size={18} /></span>Rename
                </m3e-button>
              {/if}
              {#if selected.type === 'elyby'}
                <m3e-button variant="text" onclick={() => open('https://ely.by/skins')}>
                  <span slot="icon"><Icon name="open" size={18} /></span>Change skin
                </m3e-button>
              {/if}
              <m3e-button variant="text" onclick={copyName}>
                <span slot="icon"><Icon name="copy" size={18} /></span>Copy name
              </m3e-button>
              <span class="grow"></span>
              {#if confirmRemove}
                <m3e-button class="danger" variant="filled" onclick={remove}>Remove account</m3e-button>
                <m3e-button variant="text" onclick={() => (confirmRemove = false)}>Keep</m3e-button>
              {:else}
                <m3e-icon-button aria-label="Remove {selected.name}" onclick={remove}>
                  <Icon name="trash" size={20} />
                </m3e-icon-button>
              {/if}
            </div>
            {#if confirmRemove && selected.type === 'elyby'}
              <p class="hint muted">This also signs XT Launcher out of this Ely.by account on this PC.</p>
            {/if}
          </div>
        {/key}
      {:else}
        <div class="add-pane">
          <div class="add-title">Add an account</div>

          <div class="methods" role="tablist">
            <button class="method" class:on={method === 'elyby'} role="tab" aria-selected={method === 'elyby'} onclick={() => startAdd('elyby')}>
              <span class="logo ely">E</span>
              <span class="m-text">
                <span class="m-name">Ely.by <span class="rec">Recommended</span></span>
                <span class="m-desc muted">Skins, capes and online play</span>
              </span>
            </button>
            <button class="method" class:on={method === 'offline'} role="tab" aria-selected={method === 'offline'} onclick={() => startAdd('offline')}>
              <span class="logo off"><Icon name="person" size={22} /></span>
              <span class="m-text">
                <span class="m-name">Offline</span>
                <span class="m-desc muted">Just a name, no sign-in</span>
              </span>
            </button>
            <button class="method" disabled aria-disabled="true">
              <span class="logo ms" aria-hidden="true">
                <svg viewBox="0 0 20 20" width="20" height="20"><rect x="0" y="0" width="9" height="9" fill="#f25022" /><rect x="11" y="0" width="9" height="9" fill="#7fba00" /><rect x="0" y="11" width="9" height="9" fill="#00a4ef" /><rect x="11" y="11" width="9" height="9" fill="#ffb900" /></svg>
              </span>
              <span class="m-text">
                <span class="m-name">Microsoft <span class="soon">Soon</span></span>
                <span class="m-desc muted">Your Java Edition purchase</span>
              </span>
            </button>
          </div>

          {#if method === 'elyby'}
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
                <div class="row-actions">
                  {#if elyFlow.url}<m3e-button variant="tonal" onclick={() => open(elyFlow.url)}>Open the page again</m3e-button>{/if}
                  <m3e-button variant="text" onclick={cancelAuth}>Cancel</m3e-button>
                </div>
              {:else}
                <div class="row-actions">
                  <m3e-button variant="filled" onclick={signInElyby}>
                    <span slot="icon"><Icon name="open" size={18} /></span>Continue in browser
                  </m3e-button>
                  <m3e-button variant="text" onclick={() => open('https://account.ely.by/register')}>Create an Ely.by account</m3e-button>
                </div>
                <p class="hint muted">XT Launcher never sees your password. You can revoke access any time on ely.by.</p>
              {/if}
              {#if elyFlow.error}<p class="error" role="alert">{elyFlow.error}</p>{/if}
            </div>
          {:else}
            <form class="flow offline" onsubmit={addOffline}>
              <SkinView name={previewName} unit={4} />
              <div class="offline-form">
                <m3e-form-field variant="outlined" class="field" error={!!nameProblem}>
                  <label slot="label" for="offline-name">Player name</label>
                  <input id="offline-name" bind:this={nameInput} bind:value={name} maxlength="16" autocomplete="off" spellcheck="false" />
                  <span slot="suffix-text" class="count">{trimmed.length}/16</span>
                </m3e-form-field>
                <p class="hint" class:bad={!!nameProblem}>
                  {nameProblem || 'If this name has an Ely.by skin, you will see it here and in game on supporting servers.'}
                </p>
                <div class="row-actions">
                  <m3e-button type="submit" variant="filled" disabled={!nameOk}>
                    <span slot="icon"><Icon name="add" size={18} /></span>Add offline account
                  </m3e-button>
                </div>
              </div>
            </form>
          {/if}
        </div>
      {/if}
    </section>
  </div>
</Panel>

<style>
  .split {
    display: grid;
    grid-template-columns: 236px 1fr;
    gap: 16px;
    min-height: 380px;
  }

  /* ---- Left ---- */
  .side {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-right: 16px;
    border-right: 1px solid color-mix(in srgb, var(--md-sys-color-outline-variant, #504442) 55%, transparent);
    min-width: 0;
  }
  .side-title {
    font-size: 12px;
    font-weight: 650;
    letter-spacing: 0.03em;
    padding: 0 6px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    max-height: 300px;
  }
  .list li {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: 16px;
    transition: box-shadow 160ms;
  }
  .list li:global(.dragging) {
    z-index: 3;
    box-shadow: 0 12px 30px rgb(0 0 0 / 0.45);
    background: var(--md-sys-color-surface-container-high, #2b2727);
  }
  .grip {
    all: unset;
    box-sizing: border-box;
    width: 22px;
    align-self: stretch;
    flex: none;
    display: grid;
    place-items: center;
    cursor: grab;
    touch-action: none;
    border-radius: 16px 0 0 16px;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
    opacity: 0;
    transition: opacity 120ms;
  }
  .list li:hover .grip,
  .grip:focus-visible {
    opacity: 0.9;
  }
  .grip:active {
    cursor: grabbing;
  }
  .grip:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
    outline-offset: -2px;
    opacity: 1;
  }
  .item {
    all: unset;
    box-sizing: border-box;
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px;
    border-radius: 16px;
    cursor: pointer;
    transition: background 120ms;
  }
  .item:hover {
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 55%, transparent);
  }
  .item.sel {
    background: color-mix(in srgb, var(--md-sys-color-primary-container, #8f2410) 62%, transparent);
  }
  .item:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
  }
  .item-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 3px;
  }
  .item-name {
    max-width: 100%;
    font-size: 14px;
    font-weight: 650;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dot {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 50%;
    background: var(--md-sys-color-primary, #ffb4a6);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--md-sys-color-primary, #ffb4a6) 25%, transparent);
  }
  .none {
    margin: 4px 6px;
    font-size: 13px;
  }
  .add {
    margin-top: auto;
    width: 100%;
  }

  .kind {
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

  /* ---- Right ---- */
  .main {
    min-width: 0;
  }
  .detail {
    display: flex;
    flex-direction: column;
    gap: 16px;
    animation: enter 240ms cubic-bezier(0.2, 0, 0, 1);
  }
  .hero {
    display: flex;
    gap: 24px;
    align-items: flex-start;
    padding: 18px;
    border-radius: 22px;
    background:
      radial-gradient(120% 90% at 0% 100%, color-mix(in srgb, var(--md-sys-color-primary, #c23a24) 18%, transparent), transparent 60%),
      color-mix(in srgb, var(--md-sys-color-surface-container-high, #2d2929) 70%, transparent);
  }
  .hero-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }
  .big-name {
    max-width: 100%;
    font-size: 26px;
    font-weight: 700;
    letter-spacing: -0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .facts {
    margin: 8px 0 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
  }
  .facts div {
    display: grid;
    grid-template-columns: 76px 1fr;
    gap: 10px;
    font-size: 13px;
  }
  dt {
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  dd {
    margin: 0;
    line-height: 1.4;
  }
  .mono {
    font-variant-numeric: tabular-nums;
    word-break: break-all;
    user-select: text;
  }
  .rename {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .field {
    width: 100%;
  }
  .actions,
  .row-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .grow {
    flex: 1;
  }
  .danger {
    --m3e-button-container-color: var(--md-sys-color-error, #ffb4ab);
    --m3e-button-label-text-color: var(--md-sys-color-on-error, #690005);
  }

  .add-pane {
    display: flex;
    flex-direction: column;
    gap: 12px;
    animation: enter 240ms cubic-bezier(0.2, 0, 0, 1);
  }
  .add-title {
    font-size: 18px;
    font-weight: 700;
  }
  .methods {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .method {
    all: unset;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border-radius: 18px;
    cursor: pointer;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 45%, transparent);
    outline: 1px solid transparent;
    transition:
      background 120ms,
      outline-color 120ms;
  }
  .method:hover:not(:disabled) {
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 85%, transparent);
  }
  .method.on {
    outline-color: color-mix(in srgb, var(--md-sys-color-primary, #c23a24) 70%, transparent);
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 85%, transparent);
  }
  .method:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
  }
  .method:disabled {
    cursor: default;
    opacity: 0.5;
  }
  .method:disabled svg {
    filter: grayscale(1);
  }
  .logo {
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    border-radius: 11px;
    font-weight: 800;
    font-size: 18px;
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
  .m-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .m-name {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    font-size: 14px;
    font-weight: 650;
  }
  .m-desc {
    font-size: 12px;
    line-height: 1.35;
  }
  .rec,
  .soon {
    font-size: 10px;
    font-weight: 650;
    padding: 2px 7px;
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
    padding: 16px;
    border-radius: 20px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-high, #2d2929) 70%, transparent);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .flow.offline {
    flex-direction: row;
    align-items: flex-start;
    gap: 20px;
  }
  .offline-form {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
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
  .hint {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .hint.bad,
  .error {
    color: var(--md-sys-color-error, #ffb4ab);
  }
  .error {
    margin: 0;
    font-size: 13px;
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translateX(6px);
    }
  }
</style>
