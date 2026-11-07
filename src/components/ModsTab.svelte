<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { onMount } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import { portal } from '../lib/actions';
  import ModBrowser from './ModBrowser.svelte';
  import { isXt, LOADER_LABEL, type Profile } from '../lib/data.svelte';
  import { inTauri } from '../lib/engine.svelte';
  import { identityOf, loadMods, modsState, removeMod, setModEnabled, type ModEntry } from '../lib/mods.svelte';
  import { showToast } from '../lib/toast.svelte';
  import { formatBytes } from '../lib/time';

  let { profile }: { profile: Profile } = $props();

  type Filter = 'all' | 'on' | 'off' | 'yours';
  let filter = $state<Filter>('all');
  let query = $state('');
  let browsing = $state(false);
  let confirming = $state('');
  let dropping = $state(false);

  const vanilla = $derived(profile.loader === 'vanilla');
  const list = $derived(modsState.profileId === profile.id ? modsState.list : []);
  const enabledCount = $derived(list.filter((m) => m.enabled).length);
  const offCount = $derived(list.length - enabledCount);
  const yoursCount = $derived(list.filter((m) => m.source !== 'xt').length);

  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return list.filter((m) => {
      if (filter === 'on' && !m.enabled) return false;
      if (filter === 'off' && m.enabled) return false;
      if (filter === 'yours' && m.source === 'xt') return false;
      if (!q) return true;
      return [m.name, m.modId, m.author, m.description].some((t) => t.toLowerCase().includes(q));
    });
  });

  $effect(() => {
    if (inTauri && !vanilla) void loadMods(profile.id);
  });

  async function toggle(mod: ModEntry, on: boolean) {
    try {
      await setModEnabled(profile.id, mod, on);
    } catch (err) {
      showToast(String(err));
      void loadMods(profile.id, true);
    }
  }

  async function remove(mod: ModEntry) {
    if (confirming !== mod.file) {
      confirming = mod.file;
      setTimeout(() => confirming === mod.file && (confirming = ''), 3500);
      return;
    }
    confirming = '';
    try {
      await removeMod(profile.id, mod);
      showToast(`Deleted ${mod.name}`);
    } catch (err) {
      showToast(String(err), 4200);
    }
  }

  const openFolder = () => invoke('open_dir', { kind: 'mods', profileId: profile.id }).catch((e) => showToast(String(e)));
  const modrinth = (slug: string) => void openUrl(`https://modrinth.com/mod/${slug}`);

  // Drop .jar files on the window to add them
  onMount(() => {
    if (!inTauri) return;
    let off: (() => void) | undefined;
    let gone = false;
    getCurrentWebview()
      .onDragDropEvent(async (event) => {
        const p = event.payload;
        if (p.type === 'enter' || p.type === 'over') dropping = !vanilla && !browsing;
        else if (p.type === 'leave') dropping = false;
        else if (p.type === 'drop') {
          dropping = false;
          if (vanilla || browsing) return;
          try {
            const added = await invoke<number>('add_mod_files', { profileId: profile.id, paths: p.paths });
            showToast(added ? `Added ${added} ${added === 1 ? 'mod' : 'mods'}` : 'Only .jar files that are not already here are added');
            if (added) await loadMods(profile.id, true);
          } catch (err) {
            showToast(String(err));
          }
        }
      })
      .then((un) => (gone ? un() : (off = un)))
      .catch(() => {});
    return () => {
      gone = true;
      off?.();
    };
  });

  const sourceLabel = (m: ModEntry) => (m.source === 'xt' ? 'XT' : m.source === 'modrinth' ? 'Modrinth' : 'Added by you');
</script>

{#if browsing}
  <ModBrowser {profile} onback={() => (browsing = false)} />
{:else if vanilla}
  <div class="empty">
    <Icon name="extension" size={32} />
    <p>Vanilla profiles don't use mods</p>
    <span class="muted">Edit this profile and pick Fabric, Quilt, Forge or NeoForge to add mods.</span>
  </div>
{:else}
  <div class="mods">
    <div class="top">
      <m3e-form-field variant="outlined" class="find">
        <span slot="prefix" class="pre"><Icon name="search" size={20} /></span>
        <label slot="label" for="mod-q">Search your mods</label>
        <input id="mod-q" bind:value={query} autocomplete="off" spellcheck="false" />
      </m3e-form-field>
      <m3e-button variant="filled" onclick={() => (browsing = true)}>
        <span slot="icon"><Icon name="download" size={20} /></span>Browse Modrinth
      </m3e-button>
    </div>

    <div class="bar">
      <div class="filters" role="tablist" aria-label="Filter mods">
        <button role="tab" aria-selected={filter === 'all'} class:on={filter === 'all'} onclick={() => (filter = 'all')}>All {list.length}</button>
        <button role="tab" aria-selected={filter === 'on'} class:on={filter === 'on'} onclick={() => (filter = 'on')}>On {enabledCount}</button>
        <button role="tab" aria-selected={filter === 'off'} class:on={filter === 'off'} onclick={() => (filter = 'off')}>Off {offCount}</button>
        <button role="tab" aria-selected={filter === 'yours'} class:on={filter === 'yours'} onclick={() => (filter = 'yours')}>Yours {yoursCount}</button>
      </div>
      <span class="grow"></span>
      <m3e-button variant="text" size="small" onclick={openFolder}>
        <span slot="icon"><Icon name="folder" size={18} /></span>Open folder
      </m3e-button>
    </div>

    {#if isXt(profile)}
      <p class="note muted">
        XT keeps its {LOADER_LABEL[profile.loader]} mods up to date. Switch any of them off, and add your own: those are never touched.
      </p>
    {/if}

    {#if modsState.loading && !list.length}
      <ul class="list">
        {#each Array(4) as _, i (i)}
          <li class="row ghost"><span class="skeleton"></span></li>
        {/each}
      </ul>
    {:else if modsState.error}
      <div class="empty bad">
        <Icon name="warning" size={28} />
        <p>Couldn't read the mods folder</p>
        <span>{modsState.error}</span>
      </div>
    {:else if !list.length}
      <div class="empty">
        <Icon name="extension" size={32} />
        <p>No mods yet</p>
        <span class="muted">Browse Modrinth, or drop .jar files here.</span>
      </div>
    {:else if !shown.length}
      <div class="empty">
        <Icon name="search" size={28} />
        <p>Nothing matches</p>
      </div>
    {:else}
      <ul class="list">
        {#each shown as m (m.file)}
          {@const id = identityOf(m)}
          <li class="row" class:off={!m.enabled}>
            {#if m.icon}
              <img class="ico" src={m.icon} alt="" draggable="false" />
            {:else if id?.iconUrl}
              <img class="ico" src={id.iconUrl} alt="" loading="lazy" draggable="false" />
            {:else}
              <span class="ico letter">{m.name.slice(0, 1).toUpperCase()}</span>
            {/if}
            <div class="text">
              <div class="head">
                <strong>{m.name}</strong>
                {#if m.version}<span class="ver">{m.version}</span>{/if}
                <span class="tag" class:xt={m.source === 'xt'}>{sourceLabel(m)}</span>
              </div>
              <div class="desc muted">
                {m.description || (m.author ? `by ${m.author}` : `${m.file} · ${formatBytes(m.size)}`)}
              </div>
            </div>
            {#if id?.slug}
              <m3e-icon-button aria-label="View {m.name} on Modrinth" onclick={() => modrinth(id.slug)}>
                <Icon name="open" size={18} />
              </m3e-icon-button>
            {/if}
            {#if m.source !== 'xt'}
              <m3e-button
                variant={confirming === m.file ? 'filled' : 'text'}
                size="small"
                aria-label="Delete {m.name}"
                onclick={() => remove(m)}
              >
                {#if confirming === m.file}Delete?{:else}<Icon name="trash" size={18} />{/if}
              </m3e-button>
            {/if}
            <m3e-switch
              checked={m.enabled}
              aria-label={m.enabled ? `Switch ${m.name} off` : `Switch ${m.name} on`}
              onchange={(e: Event) => toggle(m, (e.currentTarget as HTMLElement & { checked: boolean }).checked)}
            ></m3e-switch>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

{#if dropping}
  <div class="drop" use:portal aria-hidden="true">
    <div>
      <Icon name="download" size={36} />
      <strong>Drop to add to {profile.name}</strong>
      <span>.jar files go straight into the mods folder</span>
    </div>
  </div>
{/if}

<style>
  .mods {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .find {
    flex: 1;
  }
  .pre {
    display: grid;
    place-items: center;
    padding-left: 4px;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .grow {
    flex: 1;
  }
  .filters {
    display: inline-flex;
    padding: 3px;
    gap: 2px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 55%, transparent);
  }
  .filters button {
    all: unset;
    cursor: pointer;
    padding: 6px 14px;
    border-radius: 999px;
    font-size: 13px;
    font-weight: 600;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
    font-variant-numeric: tabular-nums;
    transition:
      background 140ms,
      color 140ms;
  }
  .filters button:hover {
    color: var(--md-sys-color-on-surface, #ece0dd);
  }
  .filters button.on {
    background: var(--md-sys-color-secondary-container, #5d3f3a);
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
  }
  .filters button:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
  }
  .note {
    margin: 0 4px;
    font-size: 12.5px;
    line-height: 1.45;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px 8px 8px;
    border-radius: 18px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 55%, transparent);
    transition: opacity 160ms;
    animation: rise 220ms cubic-bezier(0.2, 0, 0, 1) both;
  }
  .row.off {
    opacity: 0.55;
  }
  .row.off:hover {
    opacity: 0.85;
  }
  .row.ghost {
    height: 60px;
    padding: 0;
    overflow: hidden;
  }
  .skeleton {
    width: 100%;
    height: 100%;
    background: linear-gradient(
      100deg,
      transparent 20%,
      color-mix(in srgb, var(--md-sys-color-on-surface, #fff) 8%, transparent) 50%,
      transparent 80%
    );
    background-size: 220% 100%;
    animation: shimmer 1.4s linear infinite;
  }
  .ico {
    width: 44px;
    height: 44px;
    flex: none;
    border-radius: 13px;
    object-fit: cover;
    image-rendering: auto;
    background: var(--md-sys-color-surface-container-high, #2b2727);
  }
  .ico.letter {
    display: grid;
    place-items: center;
    font-weight: 700;
    font-size: 18px;
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
    background: var(--md-sys-color-secondary-container, #5d3f3a);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .head strong {
    font-size: 14.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ver {
    font-size: 12px;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
    white-space: nowrap;
    flex: none;
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tag {
    flex: none;
    font-size: 11px;
    font-weight: 650;
    padding: 2px 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--md-sys-color-on-surface, #fff) 10%, transparent);
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .tag.xt {
    background: var(--md-sys-color-primary-container, #8f2410);
    color: var(--md-sys-color-on-primary-container, #ffdad4);
  }
  .desc {
    font-size: 12.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-top: 1px;
  }

  .empty {
    display: grid;
    justify-items: center;
    gap: 3px;
    padding: 36px 16px;
    text-align: center;
    border-radius: 20px;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 38%, transparent);
  }
  .empty p {
    margin: 8px 0 0;
    font-weight: 650;
    color: var(--md-sys-color-on-surface, #ece0dd);
  }
  .empty.bad {
    color: var(--md-sys-color-error, #ffb4ab);
  }

  .drop {
    position: fixed;
    inset: 0;
    z-index: 70;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 0.55);
    -webkit-backdrop-filter: blur(6px);
    backdrop-filter: blur(6px);
    pointer-events: none;
    animation: fade 140ms ease-out;
  }
  .drop div {
    display: grid;
    justify-items: center;
    gap: 6px;
    padding: 40px 56px;
    border-radius: 32px;
    border: 2px dashed var(--md-sys-color-primary, #c23a24);
    color: #fff;
  }
  .drop strong {
    font-size: 18px;
  }
  .drop span {
    opacity: 0.75;
    font-size: 13px;
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
  @keyframes shimmer {
    to {
      background-position: -220% 0;
    }
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .row,
    .skeleton,
    .drop {
      animation: none;
    }
  }
</style>
