<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import Panel from './Panel.svelte';
  import {
    LOADER_LABEL,
    allProfiles,
    data,
    deleteProfile,
    duplicateProfile,
    newProfile,
    profileSubtitle,
    saveProfile,
    type Loader,
    type Profile,
  } from '../lib/data.svelte';
  import { loadVersionList } from '../lib/engine.svelte';
  import { versions } from '../lib/versions.svelte';

  type ValueEl = HTMLElement & { value: string };
  type CheckEl = HTMLElement & { checked: boolean };
  const val = (e: Event) => (e.currentTarget as ValueEl).value;
  const chk = (e: Event) => (e.currentTarget as CheckEl).checked;

  let draft = $state<Profile | null>(null);
  let isNew = $state(false);
  let confirmDelete = $state(false);
  let snapshots = $state(false);
  let ramOverride = $state(false);
  let ramGiB = $state(4);
  let error = $state('');

  const LOADERS = Object.keys(LOADER_LABEL) as Loader[];

  onMount(() => void loadVersionList(false));

  function edit(p: Profile) {
    draft = { ...p };
    isNew = false;
    confirmDelete = false;
    error = '';
    ramOverride = p.ramMb != null;
    ramGiB = p.ramMb ? p.ramMb / 1024 : 4;
  }

  function create() {
    draft = newProfile();
    isNew = true;
    confirmDelete = false;
    error = '';
    ramOverride = false;
    ramGiB = 4;
  }

  function copy(id: string) {
    const newId = duplicateProfile(id);
    const p = data.profiles.find((x) => x.id === newId);
    if (p) edit(p);
  }

  function toggleSnapshots(on: boolean) {
    snapshots = on;
    void loadVersionList(on);
  }

  function save(e: Event) {
    e.preventDefault();
    if (!draft) return;
    const name = draft.name.trim();
    if (!name) {
      error = 'Give the profile a name.';
      return;
    }
    saveProfile({
      ...draft,
      name,
      ramMb: ramOverride ? Math.round(ramGiB * 1024) : null,
      jvmArgs: (draft.jvmArgs ?? '').trim(),
    });
    data.profileId = draft.id;
    draft = null;
  }

  function remove() {
    if (!draft) return;
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    deleteProfile(draft.id);
    draft = null;
  }

  const versionOptions = $derived(versions.list.filter((v) => v.kind === 'release' || snapshots));
</script>

<Panel title={draft ? (isNew ? 'New profile' : 'Edit profile') : 'Profiles'}>
  {#if draft}
    <form class="editor" onsubmit={save}>
      <m3e-form-field variant="outlined" class="field">
        <label slot="label" for="p-name">Name</label>
        <input id="p-name" bind:value={draft.name} maxlength="32" autocomplete="off" />
      </m3e-form-field>

      <div class="two">
        <m3e-form-field variant="outlined" class="field">
          <label slot="label" for="p-loader">Loader</label>
          <m3e-select id="p-loader" value={draft.loader} onchange={(e: Event) => draft && (draft.loader = val(e) as Loader)}>
            {#each LOADERS as l (l)}
              <m3e-option value={l}>{LOADER_LABEL[l]}</m3e-option>
            {/each}
          </m3e-select>
        </m3e-form-field>

        <m3e-form-field variant="outlined" class="field">
          <label slot="label" for="p-mc">Minecraft version</label>
          <m3e-select id="p-mc" value={draft.mc} onchange={(e: Event) => draft && (draft.mc = val(e))}>
            <m3e-option value="latest">Latest release{versions.latestRelease ? ` (${versions.latestRelease})` : ''}</m3e-option>
            {#each versionOptions as v (v.id)}
              <m3e-option value={v.id}>{v.id}</m3e-option>
            {/each}
          </m3e-select>
        </m3e-form-field>
      </div>

      <label class="sw">
        <span>Include snapshots</span>
        <m3e-switch checked={snapshots} onchange={(e: Event) => toggleSnapshots(chk(e))}></m3e-switch>
      </label>

      <label class="sw">
        <span>Use its own memory</span>
        <m3e-switch checked={ramOverride} onchange={(e: Event) => (ramOverride = chk(e))}></m3e-switch>
      </label>
      {#if ramOverride}
        <div class="mem">
          <m3e-slider min="1" max="16" step="0.5" labelled class="slider" aria-label="Memory in GiB">
            <m3e-slider-thumb value={ramGiB} oninput={(e: Event) => (ramGiB = Number(val(e)))}></m3e-slider-thumb>
          </m3e-slider>
          <span class="muted amount">{ramGiB} GiB</span>
        </div>
      {/if}

      <m3e-form-field variant="outlined" class="field">
        <label slot="label" for="p-jvm">Extra JVM arguments</label>
        <input id="p-jvm" bind:value={draft.jvmArgs} spellcheck="false" autocomplete="off" />
      </m3e-form-field>

      {#if error}<p class="error" role="alert">{error}</p>{/if}

      <div class="actions">
        {#if !isNew}
          <m3e-button variant="text" onclick={remove}>{confirmDelete ? 'Confirm delete' : 'Delete'}</m3e-button>
        {/if}
        <span class="grow"></span>
        <m3e-button variant="text" onclick={() => (draft = null)}>Cancel</m3e-button>
        <m3e-button type="submit" variant="filled">Save</m3e-button>
      </div>
    </form>
  {:else}
    <ul class="list">
      {#each allProfiles() as p (p.id)}
        <li class:active={p.id === data.profileId}>
          <button class="row" onclick={() => (data.profileId = p.id)}>
            <span class="chip-icon" class:xt={p.kind !== 'custom'}><Icon name="rocket" size={20} /></span>
            <span class="text">
              <span class="title">{p.name}</span>
              <span class="sub muted">{profileSubtitle(p)}</span>
            </span>
            {#if p.kind === 'xt'}
              <span class="tag">Optimized</span>
            {:else if p.kind === 'xt-plus'}
              <span class="tag">Optimized+</span>
            {/if}
            {#if p.id === data.profileId}<Icon name="check" size={20} />{/if}
          </button>
          <m3e-icon-button aria-label="Duplicate {p.name}" onclick={() => copy(p.id)}>
            <Icon name="copy" size={20} />
          </m3e-icon-button>
          {#if p.kind === 'custom'}
            <m3e-icon-button aria-label="Edit {p.name}" onclick={() => edit(p)}>
              <Icon name="edit" size={20} />
            </m3e-icon-button>
          {/if}
        </li>
      {/each}
    </ul>
    <div class="new">
      <m3e-button variant="tonal" onclick={create}>
        <span slot="icon"><Icon name="add" size={20} /></span>New profile
      </m3e-button>
    </div>
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
    gap: 2px;
    padding-right: 4px;
    border-radius: 20px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 55%, transparent);
  }
  li.active {
    background: color-mix(in srgb, var(--md-sys-color-primary-container, #8f2410) 70%, transparent);
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
  .chip-icon {
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: 14px;
    background: var(--md-sys-color-secondary-container, #5d3f3a);
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
  }
  .chip-icon.xt {
    background: var(--md-sys-color-primary-container, #8f2410);
    color: var(--md-sys-color-on-primary-container, #ffdad4);
  }
  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .title {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    font-size: 12.5px;
  }
  .tag {
    font-size: 11.5px;
    font-weight: 600;
    padding: 3px 9px;
    border-radius: 999px;
    background: var(--md-sys-color-tertiary-container, #5b4a1f);
    color: var(--md-sys-color-on-tertiary-container, #ffe08c);
  }
  .new {
    margin-top: 10px;
  }

  .editor {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-top: 6px;
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .field {
    width: 100%;
  }
  .sw {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 14px;
    font-weight: 500;
    padding: 2px 4px;
  }
  .mem {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .slider {
    flex: 1;
  }
  .amount {
    min-width: 54px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
  }
  .grow {
    flex: 1;
  }
  .error {
    margin: 0;
    color: var(--md-sys-color-error, #ffb4ab);
    font-size: 13px;
  }
</style>
