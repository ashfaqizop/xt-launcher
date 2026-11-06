<script lang="ts">
  import Icon from '../lib/Icon.svelte';
  import Panel from './Panel.svelte';
  import { ACCOUNT_LABEL, addOfflineAccount, data, removeAccount } from '../lib/data.svelte';
  import { showToast } from '../lib/toast.svelte';

  let adding = $state(false);
  let name = $state('');
  let error = $state('');

  function submit(e: Event) {
    e.preventDefault();
    const err = addOfflineAccount(name);
    if (err) {
      error = err;
      return;
    }
    name = '';
    error = '';
    adding = false;
  }
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
          <m3e-icon-button aria-label="Remove {a.name}" onclick={() => removeAccount(a.id)}>
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
    <button class="opt" disabled onclick={() => showToast('Microsoft sign-in is coming soon')}>
      <Icon name="person" /> <span>Microsoft</span><em>Soon</em>
    </button>
    <button class="opt" disabled onclick={() => showToast('Ely.by sign-in is coming soon')}>
      <Icon name="person" /> <span>Ely.by</span><em>Soon</em>
    </button>
    <button class="opt" class:on={adding} onclick={() => (adding = !adding)}>
      <Icon name="personAdd" /> <span>Offline</span>
    </button>
  </div>

  {#if adding}
    <form class="form" onsubmit={submit}>
      <m3e-form-field variant="outlined" class="field">
        <label slot="label" for="offline-name">Username</label>
        <input id="offline-name" bind:value={name} maxlength="16" autocomplete="off" spellcheck="false" />
      </m3e-form-field>
      <m3e-button type="submit" variant="filled">Add</m3e-button>
    </form>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <p class="hint muted">Offline accounts work without signing in, but can't join online-mode servers.</p>
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
  .opt:hover:not(:disabled) {
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 90%, transparent);
  }
  .opt.on {
    background: var(--md-sys-color-secondary-container, #5d3f3a);
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
  }
  .opt:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .opt:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
  }
  em {
    font-style: normal;
    font-size: 11px;
    font-weight: 500;
    opacity: 0.8;
  }
  .form {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-top: 14px;
  }
  .field {
    flex: 1;
  }
  .error {
    color: var(--md-sys-color-error, #ffb4ab);
    margin: 8px 4px 0;
    font-size: 13px;
  }
  .hint {
    margin: 10px 4px 0;
    font-size: 12.5px;
  }
</style>
