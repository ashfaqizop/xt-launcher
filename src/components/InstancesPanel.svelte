<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import Panel from './Panel.svelte';
  import {
    LOADER_LABEL,
    TEMPLATES,
    data,
    deleteProfile,
    duplicateProfile,
    isXt,
    profileFromTemplate,
    profileSubtitle,
    saveProfile,
    type Loader,
    type Profile,
    type ProfileKind,
  } from '../lib/data.svelte';
  import { loadVersionList } from '../lib/engine.svelte';
  import { nav } from '../lib/nav.svelte';
  import { versions } from '../lib/versions.svelte';
  import { maxGameMemoryGiB } from '../lib/system.svelte';

  type ValueEl = HTMLElement & { value: string };
  type CheckEl = HTMLElement & { checked: boolean };
  const val = (e: Event) => (e.currentTarget as ValueEl).value;
  const chk = (e: Event) => (e.currentTarget as CheckEl).checked;

  type Step = 'list' | 'choose' | 'edit';
  let step = $state<Step>('list');
  let draft = $state<Profile | null>(null);
  let isNew = $state(false);
  let confirmDelete = $state(false);
  let snapshots = $state(false);
  let ramOverride = $state(false);
  let ramGiB = $state(4);
  let error = $state('');

  const LOADERS = Object.keys(LOADER_LABEL) as Loader[];

  onMount(() => {
    void loadVersionList(false);
    // Opened from "Create a profile" elsewhere, or there's nothing to list yet
    if (nav.createProfile || data.profiles.length === 0) {
      nav.createProfile = false;
      step = 'choose';
    }
  });

  function openEditor(p: Profile, fresh: boolean) {
    draft = { ...p };
    isNew = fresh;
    confirmDelete = false;
    error = '';
    ramOverride = p.ramMb != null;
    ramGiB = p.ramMb ? p.ramMb / 1024 : 4;
    step = 'edit';
  }

  const choose = (kind: ProfileKind) => openEditor(profileFromTemplate(kind), true);

  function copy(id: string) {
    const newId = duplicateProfile(id);
    const p = data.profiles.find((x) => x.id === newId);
    if (p) openEditor(p, false);
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
    step = 'list';
  }

  function remove() {
    if (!draft) return;
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    deleteProfile(draft.id);
    draft = null;
    step = data.profiles.length ? 'list' : 'choose';
  }

  function back() {
    draft = null;
    step = data.profiles.length ? 'list' : 'choose';
  }

  const versionOptions = $derived(versions.list.filter((v) => v.kind === 'release' || snapshots));
  const title = $derived(step === 'choose' ? 'New profile' : step === 'edit' ? (isNew ? 'New profile' : 'Edit profile') : 'Profiles');
  const tier = $derived(draft ? TEMPLATES.find((t) => t.kind === draft!.kind) : undefined);
</script>

<Panel {title}>
  {#if step === 'choose'}
    <p class="lead muted">Start from a recommended setup, or build your own.</p>
    <div class="templates">
      {#each TEMPLATES as t (t.kind)}
        <button class="template" class:rec={t.recommended} onclick={() => choose(t.kind)}>
          <span class="t-icon" class:xt={t.kind !== 'custom'}>
            <Icon name={t.kind === 'custom' ? 'edit' : 'rocket'} size={22} />
          </span>
          <span class="t-text">
            <span class="t-head">
              <span class="t-name">{t.name}</span>
              {#if t.recommended}<span class="tag">Recommended</span>{/if}
            </span>
            <span class="t-sum muted">{t.summary}</span>
            {#if t.mods}<span class="t-mods">{t.mods}</span>{/if}
          </span>
        </button>
      {/each}
    </div>
    {#if data.profiles.length}
      <div class="foot"><m3e-button variant="text" onclick={() => (step = 'list')}>Back to profiles</m3e-button></div>
    {/if}
  {:else if step === 'edit' && draft}
    <form class="editor" onsubmit={save}>
      {#if tier && isXt(draft)}
        <div class="tier">
          <span class="t-icon xt"><Icon name="rocket" size={20} /></span>
          <span class="t-text">
            <span class="t-name">{tier.name}</span>
            <span class="t-sum muted">Fabric, curated mods kept up to date, ZGC</span>
          </span>
        </div>
      {/if}

      <m3e-form-field variant="outlined" class="field">
        <label slot="label" for="p-name">Name</label>
        <input id="p-name" bind:value={draft.name} maxlength="32" autocomplete="off" />
      </m3e-form-field>

      <div class="two" class:one={isXt(draft)}>
        {#if !isXt(draft)}
          <m3e-form-field variant="outlined" class="field">
            <label slot="label" for="p-loader">Loader</label>
            <m3e-select id="p-loader" value={draft.loader} onchange={(e: Event) => draft && (draft.loader = val(e) as Loader)}>
              {#each LOADERS as l (l)}
                <m3e-option value={l}>{LOADER_LABEL[l]}</m3e-option>
              {/each}
            </m3e-select>
          </m3e-form-field>
        {/if}

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

      {#if !isXt(draft)}
        <label class="sw">
          <span>Use ZGC (smoother frame times)</span>
          <m3e-switch checked={!!draft.zgc} onchange={(e: Event) => draft && (draft.zgc = chk(e))}></m3e-switch>
        </label>
      {/if}

      <label class="sw">
        <span>Use its own memory</span>
        <m3e-switch checked={ramOverride} onchange={(e: Event) => (ramOverride = chk(e))}></m3e-switch>
      </label>
      {#if ramOverride}
        <div class="mem">
          <m3e-slider min="1" max={maxGameMemoryGiB()} step="0.5" labelled class="slider" aria-label="Memory in GiB">
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
        <m3e-button variant="text" onclick={back}>Cancel</m3e-button>
        <m3e-button type="submit" variant="filled">{isNew ? 'Create' : 'Save'}</m3e-button>
      </div>
    </form>
  {:else}
    <ul class="list">
      {#each data.profiles as p (p.id)}
        <li class:active={p.id === data.profileId}>
          <button class="row" onclick={() => (data.profileId = p.id)}>
            <span class="chip-icon" class:xt={isXt(p)}><Icon name="rocket" size={20} /></span>
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
          <m3e-icon-button aria-label="Edit {p.name}" onclick={() => openEditor(p, false)}>
            <Icon name="edit" size={20} />
          </m3e-icon-button>
        </li>
      {/each}
    </ul>
    <div class="foot">
      <m3e-button variant="tonal" onclick={() => (step = 'choose')}>
        <span slot="icon"><Icon name="add" size={20} /></span>New profile
      </m3e-button>
    </div>
  {/if}
</Panel>

<style>
  .lead {
    margin: 0 4px 12px;
    font-size: 13.5px;
  }
  .templates {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .template,
  .tier {
    all: unset;
    box-sizing: border-box;
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 14px 16px 14px 14px;
    border-radius: 20px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 55%, transparent);
  }
  .template {
    cursor: pointer;
    transition: background 120ms;
  }
  .template:hover {
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 90%, transparent);
  }
  .template.rec {
    outline: 1px solid color-mix(in srgb, var(--md-sys-color-primary, #c23a24) 45%, transparent);
  }
  .template:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
  }
  .tier {
    align-items: center;
  }
  .t-icon {
    width: 44px;
    height: 44px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 14px;
    background: var(--md-sys-color-secondary-container, #5d3f3a);
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
  }
  .t-icon.xt {
    background: var(--md-sys-color-primary-container, #8f2410);
    color: var(--md-sys-color-on-primary-container, #ffdad4);
  }
  .t-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .t-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .t-name {
    font-size: 15.5px;
    font-weight: 650;
  }
  .t-sum {
    font-size: 13px;
    line-height: 1.4;
  }
  .t-mods {
    font-size: 12px;
    line-height: 1.45;
    color: var(--md-sys-color-primary, #c23a24);
  }

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
    flex: none;
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
    flex: none;
    font-size: 11.5px;
    font-weight: 600;
    padding: 3px 9px;
    border-radius: 999px;
    background: var(--md-sys-color-tertiary-container, #5b4a1f);
    color: var(--md-sys-color-on-tertiary-container, #ffe08c);
  }
  .foot {
    margin-top: 12px;
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
  .two.one {
    grid-template-columns: 1fr;
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
