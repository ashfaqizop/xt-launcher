<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import Icon from '../lib/Icon.svelte';
  import ProfileIcon from '../lib/ProfileIcon.svelte';
  import Panel from './Panel.svelte';
  import ModsTab from './ModsTab.svelte';
  import Screenshots from './Screenshots.svelte';
  import { LOADER_LABEL, TEMPLATES, currentProfile, formatPlaytime, isXt, profileSubtitle } from '../lib/data.svelte';
  import { engine, inTauri, play, repairProfile } from '../lib/engine.svelte';
  import { go, goCreateProfile, goEditProfile, nav, type ProfileTab } from '../lib/nav.svelte';
  import { showToast } from '../lib/toast.svelte';
  import { lastPlayedText } from '../lib/time';

  const profile = $derived(currentProfile());
  const tier = $derived(profile ? TEMPLATES.find((t) => t.kind === profile.kind) : undefined);
  const busy = $derived(engine.phase !== 'idle');
  let confirmRepair = $state(false);

  const tab = (t: ProfileTab) => () => (nav.profileTab = t);

  function folder(kind: 'game' | 'mods' | 'screenshots') {
    if (!profile) return;
    if (!inTauri) return showToast('Folders open in the desktop app');
    invoke('open_dir', { kind, profileId: profile.id }).catch((e) => showToast(String(e)));
  }

  async function repair() {
    if (!confirmRepair) {
      confirmRepair = true;
      return;
    }
    confirmRepair = false;
    await repairProfile();
  }
</script>

<Panel title={profile ? profile.name : 'Profile'} wide>
  {#if !profile}
    <div class="none">
      <p>No profile selected</p>
      <m3e-button variant="filled" onclick={goCreateProfile}>Create a profile</m3e-button>
    </div>
  {:else}
    <m3e-tabs>
      <m3e-tab for="pt-overview" selected={nav.profileTab === 'overview'} onclick={tab('overview')}>Overview</m3e-tab>
      <m3e-tab for="pt-mods" selected={nav.profileTab === 'mods'} onclick={tab('mods')}>Mods</m3e-tab>
      <m3e-tab for="pt-shots" selected={nav.profileTab === 'screenshots'} onclick={tab('screenshots')}>Screenshots</m3e-tab>

      <!-- ================= Overview ================= -->
      <m3e-tab-panel id="pt-overview">
        {#if nav.profileTab === 'overview'}
          <div class="tab">
            <section class="hero" style:--accent={profile.accent || undefined} class:accented={!!profile.accent}>
              <ProfileIcon {profile} size={76} />
              <div class="who">
                <div class="name">
                  <h3>{profile.name}</h3>
                  {#if tier && isXt(profile)}<span class="tag">{tier.name}</span>{/if}
                </div>
                <div class="sub">{profileSubtitle(profile)}</div>
                <div class="facts">
                  <span><Icon name="history" size={15} />{lastPlayedText(profile.lastPlayed)}</span>
                  {#if profile.playMs}<span><Icon name="schedule" size={15} />{formatPlaytime(profile.playMs)} played</span>{/if}
                  {#if profile.ramMb}<span><Icon name="memory" size={15} />{profile.ramMb / 1024} GiB</span>{/if}
                </div>
              </div>
              <div class="cta">
                <m3e-button variant="filled" disabled={busy} onclick={play}>
                  <span slot="icon"><Icon name="play" size={22} /></span>Play
                </m3e-button>
                <m3e-button variant="tonal" onclick={() => goEditProfile(profile.id)}>
                  <span slot="icon"><Icon name="edit" size={20} /></span>Edit
                </m3e-button>
              </div>
            </section>

            <Screenshots profileId={profile.id} limit={8} layout="strip" onSeeAll={tab('screenshots')} />

            <h4>Tools</h4>
            <div class="tools">
              <button class="tool" onclick={() => folder('game')}>
                <span class="ti"><Icon name="folder" size={22} /></span>
                <span class="tt"><strong>Game folder</strong><span class="muted">Saves, options and logs</span></span>
              </button>
              <button class="tool" onclick={tab('mods')}>
                <span class="ti"><Icon name="extension" size={22} /></span>
                <span class="tt">
                  <strong>Mods</strong>
                  <span class="muted">{profile.loader === 'vanilla' ? 'Vanilla has no mods' : `Manage and add ${LOADER_LABEL[profile.loader]} mods`}</span>
                </span>
              </button>
              <button class="tool repair" class:ask={confirmRepair} disabled={busy} onclick={repair}>
                <span class="ti"><Icon name="build" size={22} /></span>
                <span class="tt">
                  {#if confirmRepair}
                    <strong>Check every file now?</strong>
                    <span class="muted">Takes a minute or two. Click again to start.</span>
                  {:else}
                    <strong>Repair profile</strong>
                    <span class="muted">Fix a game that won't start after a crash</span>
                  {/if}
                </span>
              </button>
              <button class="tool" onclick={() => go('logs')}>
                <span class="ti"><Icon name="logs" size={22} /></span>
                <span class="tt"><strong>Game log</strong><span class="muted">See what the last run printed</span></span>
              </button>
            </div>
          </div>
        {/if}
      </m3e-tab-panel>

      <!-- ================= Mods ================= -->
      <m3e-tab-panel id="pt-mods">
        {#if nav.profileTab === 'mods'}
          <div class="tab"><ModsTab {profile} /></div>
        {/if}
      </m3e-tab-panel>

      <!-- ================= Screenshots ================= -->
      <m3e-tab-panel id="pt-shots">
        {#if nav.profileTab === 'screenshots'}
          <div class="tab"><Screenshots profileId={profile.id} layout="grid" /></div>
        {/if}
      </m3e-tab-panel>
    </m3e-tabs>
  {/if}
</Panel>

<style>
  .tab {
    padding: 8px 4px 8px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .none {
    display: grid;
    justify-items: center;
    gap: 10px;
    padding: 36px 0;
  }

  .hero {
    position: relative;
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 18px;
    border-radius: 26px;
    background: color-mix(in srgb, var(--md-sys-color-primary-container, #8f2410) 42%, transparent);
    overflow: hidden;
  }
  .hero.accented {
    background: linear-gradient(
      125deg,
      color-mix(in srgb, var(--accent) 46%, transparent),
      color-mix(in srgb, var(--accent) 10%, transparent) 78%
    );
  }
  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  h3 {
    margin: 0;
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tag {
    flex: none;
    font-size: 11.5px;
    font-weight: 650;
    padding: 3px 10px;
    border-radius: 999px;
    background: var(--md-sys-color-tertiary-container, #5b4a1f);
    color: var(--md-sys-color-on-tertiary-container, #ffe08c);
  }
  .sub {
    font-size: 14px;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
    margin-top: 4px;
    font-size: 12.5px;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .facts span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .cta {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: none;
  }

  h4 {
    margin: 0 0 -6px 4px;
    font-size: 12.5px;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .tools {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
  }
  .tool {
    all: unset;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px 14px 14px;
    border-radius: 20px;
    cursor: pointer;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 55%, transparent);
    transition:
      background 140ms,
      transform 140ms;
  }
  .tool:hover:not(:disabled) {
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 95%, transparent);
    transform: translateY(-1px);
  }
  .tool:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .tool:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
  }
  .tool.ask {
    background: var(--md-sys-color-primary-container, #8f2410);
    color: var(--md-sys-color-on-primary-container, #ffdad4);
  }
  .tool.ask .muted {
    color: inherit;
    opacity: 0.8;
  }
  .ti {
    width: 44px;
    height: 44px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 14px;
    background: var(--md-sys-color-secondary-container, #5d3f3a);
    color: var(--md-sys-color-on-secondary-container, #ffdad4);
  }
  .tt {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 1px;
  }
  .tt strong {
    font-size: 14.5px;
  }
  .tt .muted {
    font-size: 12.5px;
    line-height: 1.35;
  }
</style>
