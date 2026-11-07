<script lang="ts">
  import { selectValue } from '../lib/actions';
  import { onMount } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import Panel from './Panel.svelte';
  import IconPicker from './IconPicker.svelte';
  import RamSlider from './RamSlider.svelte';
  import ProfileIcon from '../lib/ProfileIcon.svelte';
  import LoaderLogo from '../lib/LoaderLogo.svelte';
  import { sortable } from '../lib/sortable';
  import { formatBytes, lastPlayedText } from '../lib/time';
  import { showToast } from '../lib/toast.svelte';
  import {
    LOADER_LABEL,
    TEMPLATES,
    data,
    deleteProfile,
    duplicateProfile,
    formatPlaytime,
    isXt,
    moveProfile,
    profileFromTemplate,
    profileSubtitle,
    saveProfile,
    type Loader,
    type Profile,
    type ProfileKind,
  } from '../lib/data.svelte';
  import { inTauri, loadVersionList } from '../lib/engine.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { goProfile, nav } from '../lib/nav.svelte';
  import { versions } from '../lib/versions.svelte';

  type ValueEl = HTMLElement & { value: string };
  type CheckEl = HTMLElement & { checked: boolean };
  const val = (e: Event) => (e.currentTarget as ValueEl).value;
  const fmtCount = (n: number) => n.toLocaleString('en-US');
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
  let showLook = $state(false);

  const LOADERS = Object.keys(LOADER_LABEL) as Loader[];

  onMount(() => {
    void loadVersionList(false);
    // Opened from a profile's page: straight into its editor
    if (nav.editProfile) {
      const wanted = data.profiles.find((p) => p.id === nav.editProfile);
      nav.editProfile = '';
      if (wanted) {
        openEditor(wanted, false);
        return;
      }
    }
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
    showLook = false;
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
    if (mcUnsupported) {
      error = `${LOADER_LABEL[draft.loader]} doesn't support Minecraft ${resolvedMc} yet. Pick another version.`;
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

  async function remove() {
    if (!draft) return;
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    const id = draft.id;
    if (inTauri) {
      try {
        const freed = await invoke<number>('delete_profile_files', { id });
        if (freed > 0) showToast(`Deleted the profile and freed ${formatBytes(freed)}`);
      } catch (err) {
        // The game may be running; keep the profile so nothing is orphaned
        showToast(String(err), 4500);
        confirmDelete = false;
        return;
      }
    }
    deleteProfile(id);
    draft = null;
    step = data.profiles.length ? 'list' : 'choose';
  }

  function back() {
    draft = null;
    step = data.profiles.length ? 'list' : 'choose';
  }

  // ---- What the chosen loader actually supports ----------------------------

  interface Catalog {
    games: string[];
    builds: { version: string; stable: boolean }[];
    recommended: string | null;
  }
  let catalog = $state<Catalog | null>(null);
  let catalogFor = $state('');
  let catalogLoading = $state(false);
  let catalogError = $state('');

  const loader = $derived(draft?.loader ?? 'vanilla');
  const resolvedMc = $derived(draft ? (draft.mc === 'latest' ? versions.latestRelease : draft.mc) : '');

  $effect(() => {
    const l = loader;
    const mc = resolvedMc;
    if (l === 'vanilla' || !inTauri) {
      catalog = null;
      catalogError = '';
      return;
    }
    const key = l + '|' + mc;
    let alive = true;
    catalogLoading = true;
    catalogError = '';
    invoke<Catalog>('loader_catalog', { loader: l, mc: mc || null })
      .then((c) => {
        if (!alive) return;
        catalog = c;
        catalogFor = key;
      })
      .catch((err) => {
        if (!alive) return;
        catalog = null;
        catalogError = String(err);
      })
      .finally(() => alive && (catalogLoading = false));
    return () => {
      alive = false;
    };
  });

  const supported = $derived(catalog ? new Set(catalog.games) : null);
  /** Minecraft versions to offer: Mojang's list, narrowed to what the loader supports */
  const versionOptions = $derived(
    versions.list.filter((v) => (v.kind === 'release' || snapshots) && (!supported || supported.has(v.id))),
  );
  const latestSupported = $derived(!supported || !versions.latestRelease || supported.has(versions.latestRelease));
  const mcUnsupported = $derived(!!supported && !!resolvedMc && !supported.has(resolvedMc));
  const builds = $derived(catalog && catalogFor === loader + '|' + resolvedMc ? catalog.builds.slice(0, 80) : []);

  /** The version folder the launcher will install, e.g. fabric-loader-0.20.1-26.3 */
  function installedId(l: Loader, build: string, mc: string) {
    switch (l) {
      case 'fabric':
        return `fabric-loader-${build}-${mc}`;
      case 'quilt':
        return `quilt-loader-${build}-${mc}`;
      case 'forge':
        return `${mc}-forge-${build}`;
      case 'neoforge':
        return `neoforge-${build}`;
      default:
        return mc;
    }
  }
  const buildsKey = $derived(`${loader}|${resolvedMc}|${catalogLoading}|${builds.length}`);
  const mcKey = $derived(`${loader}|${supported ? supported.size : 0}|${snapshots}`);

  // If the loader doesn't support the chosen Minecraft version, move to the newest one it does
  let switchedNote = $state('');
  $effect(() => {
    if (!draft || catalogLoading || !mcUnsupported) return;
    const target = versionOptions[0]?.id;
    if (!target) return;
    switchedNote = `${LOADER_LABEL[draft.loader]} doesn't support ${resolvedMc} yet, so this profile uses ${target}, the newest version it supports.`;
    setMc(target);
  });

  const effectiveBuild = $derived(draft?.loaderVersion || catalog?.recommended || '');

  function setLoader(l: Loader) {
    if (!draft) return;
    switchedNote = '';
    draft.loader = l;
    draft.loaderVersion = '';
  }
  function setMc(mc: string) {
    if (!draft) return;
    draft.mc = mc;
    draft.loaderVersion = '';
  }
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

      <div class="identity">
        <button
          type="button"
          class="avatar"
          aria-label="Change icon and colour"
          aria-expanded={showLook}
          onclick={() => (showLook = !showLook)}
        >
          <ProfileIcon profile={draft} size={56} />
          <span class="badge"><Icon name="palette" size={14} /></span>
        </button>
        <m3e-form-field variant="outlined" class="field grow">
          <label slot="label" for="p-name">Name</label>
          <input id="p-name" bind:value={draft.name} maxlength="32" autocomplete="off" />
        </m3e-form-field>
      </div>
      {#if showLook}
        <IconPicker profile={draft} onchange={(patch) => draft && Object.assign(draft, patch)} />
      {/if}

      {#if !isXt(draft)}
        <div class="loaders" role="radiogroup" aria-label="Loader">
          {#each LOADERS as l (l)}
            <button
              type="button"
              role="radio"
              aria-checked={draft.loader === l}
              class="lchip"
              class:on={draft.loader === l}
              onclick={() => setLoader(l)}
            >
              <LoaderLogo loader={l} size={24} />
              <span>{LOADER_LABEL[l]}</span>
            </button>
          {/each}
        </div>
      {/if}

      <div class="two one">

        {#key mcKey}
        <m3e-form-field variant="outlined" class="field">
          <label slot="label" for="p-mc">Minecraft version</label>
          <m3e-select id="p-mc" use:selectValue={draft.mc} onchange={(e: Event) => setMc(val(e))}>
            {#if latestSupported}
              <m3e-option value="latest">Latest release{versions.latestRelease ? ` (${versions.latestRelease})` : ''}</m3e-option>
            {/if}
            {#each versionOptions as v (v.id)}
              <m3e-option value={v.id}>{v.id}{v.kind === 'snapshot' ? '  ·  snapshot' : ''}</m3e-option>
            {/each}
          </m3e-select>
        </m3e-form-field>
        {/key}
      </div>

      {#if draft.loader !== 'vanilla'}
        {#if !isXt(draft)}
          {#key buildsKey}
          <m3e-form-field variant="outlined" class="field">
            <label slot="label" for="p-build">{LOADER_LABEL[draft.loader]} version</label>
              <m3e-select
                id="p-build"
                use:selectValue={draft.loaderVersion || 'recommended'}
                disabled={catalogLoading || mcUnsupported || !builds.length}
                onchange={(e: Event) => draft && (draft.loaderVersion = val(e) === 'recommended' ? '' : val(e))}
              >
                <m3e-option value="recommended">
                  {catalogLoading ? 'Loading versions…' : catalog?.recommended ? `Recommended (${catalog.recommended})` : 'Recommended'}
                </m3e-option>
                {#each builds as b (b.version)}
                  <m3e-option value={b.version}>{b.version}{b.stable ? '' : '  ·  beta'}</m3e-option>
                {/each}
              </m3e-select>
          </m3e-form-field>
          {/key}
        {/if}
        <p class="build-note" class:bad={mcUnsupported || !!catalogError}>
          {#if catalogError}
            Couldn't load {LOADER_LABEL[draft.loader]} versions. Check your connection; the recommended build is used when you play.
          {:else if mcUnsupported}
            {LOADER_LABEL[draft.loader]} doesn't support Minecraft {resolvedMc} yet.
          {:else if catalogLoading}
            Checking what {LOADER_LABEL[draft.loader]} supports…
          {:else if switchedNote}
            {switchedNote}
          {:else if catalog && effectiveBuild && resolvedMc}
            Installs <span class="mono">{installedId(draft.loader, effectiveBuild, resolvedMc)}</span>
            · {fmtCount(catalog.games.length)} Minecraft versions supported
          {/if}
        </p>
      {/if}

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
        <RamSlider value={ramGiB} onchange={(v) => (ramGiB = v)} />
      {/if}

      <m3e-form-field variant="outlined" class="field">
        <label slot="label" for="p-jvm">Extra JVM arguments</label>
        <input id="p-jvm" bind:value={draft.jvmArgs} spellcheck="false" autocomplete="off" />
      </m3e-form-field>

      {#if error}<p class="error" role="alert">{error}</p>{/if}

      <div class="actions">
        {#if !isNew}
          <m3e-button variant={confirmDelete ? 'filled' : 'text'} onclick={remove}>{confirmDelete ? 'Delete profile and all its files?' : 'Delete'}</m3e-button>
        {/if}
        <span class="grow"></span>
        <m3e-button variant="text" onclick={back}>Cancel</m3e-button>
        <m3e-button type="submit" variant="filled">{isNew ? 'Create' : 'Save'}</m3e-button>
      </div>
    </form>
  {:else}
    <ul class="list" use:sortable={{ onMove: moveProfile }}>
      {#each data.profiles as p (p.id)}
        <li class:active={p.id === data.profileId}>
          <button
            type="button"
            class="grip"
            data-handle
            aria-label="Reorder {p.name}. Drag it, or hold Alt and press the up or down arrow."
          >
            <Icon name="drag" size={20} />
          </button>
          <button class="row" onclick={() => (data.profileId = p.id)}>
            <ProfileIcon profile={p} size={40} />
            <span class="text">
              <span class="title">{p.name}</span>
              <span class="sub muted">{profileSubtitle(p)}{p.playMs ? ` · played ${formatPlaytime(p.playMs)}` : ''}</span>
              <span class="sub muted when">{lastPlayedText(p.lastPlayed)}</span>
            </span>
            {#if p.kind === 'xt'}
              <span class="tag">Optimized</span>
            {:else if p.kind === 'xt-plus'}
              <span class="tag">Optimized+</span>
            {/if}
            {#if p.id === data.profileId}<Icon name="check" size={20} />{/if}
          </button>
          <m3e-icon-button
            aria-label="Open the page for {p.name}"
            onclick={() => {
              data.profileId = p.id;
              goProfile();
            }}
          >
            <Icon name="tune" size={20} />
          </m3e-icon-button>
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
  li {
    position: relative;
    transition: box-shadow 160ms;
  }
  li:global(.dragging) {
    z-index: 3;
    box-shadow: 0 14px 36px rgb(0 0 0 / 0.45);
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 100%, transparent);
  }
  .grip {
    all: unset;
    box-sizing: border-box;
    width: 28px;
    align-self: stretch;
    display: grid;
    place-items: center;
    flex: none;
    cursor: grab;
    touch-action: none;
    border-radius: 20px 0 0 20px;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
    opacity: 0.55;
    transition: opacity 120ms;
  }
  li:hover .grip,
  .grip:focus-visible {
    opacity: 1;
  }
  .grip:active {
    cursor: grabbing;
  }
  .grip:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
    outline-offset: -2px;
  }
  .when {
    font-size: 12px;
    opacity: 0.85;
  }
  .row {
    all: unset;
    box-sizing: border-box;
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 8px 10px 2px;
    cursor: pointer;
  }
  .row:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
    border-radius: 20px;
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
  .loaders {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 8px;
  }
  .lchip {
    all: unset;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 12px 4px 10px;
    border-radius: 18px;
    cursor: pointer;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 50%, transparent);
    outline: 2px solid transparent;
    outline-offset: -2px;
    transition:
      background 140ms,
      color 140ms,
      outline-color 140ms,
      transform 140ms;
  }
  .lchip:hover {
    background: var(--md-sys-color-surface-container-highest, #383434);
    transform: translateY(-1px);
  }
  .lchip.on {
    color: var(--md-sys-color-on-primary-container, #ffdad4);
    background: color-mix(in srgb, var(--md-sys-color-primary-container, #8f2410) 70%, transparent);
    outline-color: var(--md-sys-color-primary, #c23a24);
  }
  .lchip:focus-visible {
    outline-color: var(--md-sys-color-primary, #c23a24);
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
  .identity {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .identity .grow {
    flex: 1;
    min-width: 0;
  }
  .avatar {
    all: unset;
    position: relative;
    flex: none;
    cursor: pointer;
    border-radius: 18px;
    transition: transform 140ms;
  }
  .avatar:hover {
    transform: scale(1.04);
  }
  .avatar:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
    outline-offset: 3px;
  }
  .badge {
    position: absolute;
    right: -4px;
    bottom: -4px;
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--md-sys-color-primary, #c23a24);
    color: var(--md-sys-color-on-primary, #fff);
    box-shadow: 0 0 0 3px var(--md-sys-color-surface-container, #1e1b1b);
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
  .build-note {
    margin: -4px 4px 0;
    min-height: 1.4em;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .build-note.bad {
    color: var(--md-sys-color-error, #ffb4ab);
  }
  .mono {
    font-variant-numeric: tabular-nums;
    color: var(--md-sys-color-primary, #c23a24);
    user-select: text;
  }
</style>
