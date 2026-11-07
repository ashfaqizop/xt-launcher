<script lang="ts">
  import Icon from '../lib/Icon.svelte';
  import { go, goCreateProfile } from '../lib/nav.svelte';
  import { showToast } from '../lib/toast.svelte';
  import { checkForUpdates } from '../lib/updater';
  import { engine, openFolder, play } from '../lib/engine.svelte';
  import {
    ACCOUNT_LABEL,
    isXt,
    currentAccount,
    currentProfile,
    data,
    profileSubtitle,
  } from '../lib/data.svelte';

  const acc = $derived(currentAccount());
  const profile = $derived(currentProfile());

  async function refresh() {
    showToast('Checking for updates…', 1500);
    const r = await checkForUpdates();
    if (r.kind === 'latest') showToast("You're on the latest version");
    else if (r.kind === 'available') showToast(`Update v${r.version} is available. Open Settings › About to install`, 4200);
    else showToast("Couldn't reach GitHub");
  }
</script>

<section class="card glass" aria-label="Play">
  <!-- Account -->
  <button class="picker" aria-label="Choose account">
    <m3e-menu-trigger for="acc-menu"></m3e-menu-trigger>
    <m3e-avatar class="avatar">{acc ? acc.name.slice(0, 1).toUpperCase() : '+'}</m3e-avatar>
    <span class="text">
      <span class="title">{acc ? acc.name : 'Add an account'}</span>
      <span class="sub muted">{acc ? ACCOUNT_LABEL[acc.type] : 'Microsoft, Ely.by or offline'}</span>
    </span>
    <Icon name="down" size={22} />
  </button>
  <m3e-menu id="acc-menu">
    {#each data.accounts as a (a.id)}
      <m3e-menu-item onclick={() => (data.accountId = a.id)}>{a.name} · {ACCOUNT_LABEL[a.type]}</m3e-menu-item>
    {/each}
    {#if data.accounts.length}<m3e-divider></m3e-divider>{/if}
    <m3e-menu-item onclick={() => go('accounts')}>Manage accounts…</m3e-menu-item>
  </m3e-menu>

  <!-- Profile -->
  {#if profile}
    <button class="picker" aria-label="Choose profile">
      <m3e-menu-trigger for="profile-menu"></m3e-menu-trigger>
      <span class="chip-icon" class:xt={isXt(profile)}><Icon name="rocket" size={20} /></span>
      <span class="text">
        <span class="title">{profile.name}</span>
        <span class="sub muted">{profileSubtitle(profile)}</span>
      </span>
      <Icon name="down" size={22} />
    </button>
    <m3e-menu id="profile-menu">
      {#each data.profiles as p (p.id)}
        <m3e-menu-item onclick={() => (data.profileId = p.id)}>{p.name} · {profileSubtitle(p)}</m3e-menu-item>
      {/each}
      <m3e-divider></m3e-divider>
      <m3e-menu-item onclick={goCreateProfile}>New profile…</m3e-menu-item>
      <m3e-menu-item onclick={() => go('instances')}>Manage profiles…</m3e-menu-item>
    </m3e-menu>
  {:else}
    <button class="picker empty" aria-label="Create a profile" onclick={goCreateProfile}>
      <span class="chip-icon xt"><Icon name="add" size={22} /></span>
      <span class="text">
        <span class="title">Create a profile</span>
        <span class="sub muted">XT Optimized is recommended</span>
      </span>
    </button>
  {/if}

  <!-- Play (progress and controls live in the launch ribbon) -->
  <m3e-button class="play" variant={engine.phase === 'idle' ? 'filled' : 'tonal'} size="large" disabled={engine.phase !== 'idle'} onclick={play}>
    <span slot="icon" class="play-ic"><Icon name="play" size={28} /></span>
    {engine.phase === 'running' ? 'Playing' : engine.phase === 'installing' ? 'Starting…' : 'Play'}
  </m3e-button>

  <!-- Quick actions -->
  <div class="actions">
    <m3e-icon-button id="b-logs" variant="tonal" aria-label="Game logs" onclick={() => go('logs')}>
      <Icon name="logs" />
    </m3e-icon-button>
    <m3e-tooltip for="b-logs">Game logs</m3e-tooltip>

    <m3e-icon-button id="b-folder" variant="tonal" aria-label="Folders">
      <m3e-menu-trigger for="folder-menu"></m3e-menu-trigger>
      <Icon name="folder" />
    </m3e-icon-button>
    <m3e-menu id="folder-menu">
      <m3e-menu-item onclick={() => openFolder('mods')}>Open mods folder</m3e-menu-item>
      <m3e-menu-item onclick={() => openFolder('game')}>Open game folder</m3e-menu-item>
    </m3e-menu>

    <m3e-icon-button id="b-refresh" variant="tonal" aria-label="Check for updates" onclick={refresh}>
      <Icon name="refresh" />
    </m3e-icon-button>
    <m3e-tooltip for="b-refresh">Check for updates</m3e-tooltip>

    <m3e-icon-button id="b-more" variant="tonal" aria-label="More">
      <m3e-menu-trigger for="more-menu"></m3e-menu-trigger>
      <Icon name="more" />
    </m3e-icon-button>
    <m3e-menu id="more-menu" position-x="before">
      <m3e-menu-item onclick={() => go('settings', 'minecraft')}>Game settings</m3e-menu-item>
      <m3e-menu-item onclick={() => go('settings', 'launcher')}>Launcher settings</m3e-menu-item>
      <m3e-menu-item onclick={() => go('instances')}>Profiles</m3e-menu-item>
      <m3e-menu-item onclick={() => go('accounts')}>Manage accounts</m3e-menu-item>
      <m3e-divider></m3e-divider>
      <m3e-menu-item onclick={() => go('settings', 'about')}>About XT Launcher</m3e-menu-item>
    </m3e-menu>
  </div>
</section>

<style>
  .card {
    width: 340px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    animation: rise 320ms cubic-bezier(0.2, 0, 0, 1);
  }

  .picker {
    all: unset;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px 10px 10px;
    border-radius: 20px;
    cursor: pointer;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 70%, transparent);
    transition: background 120ms;
  }
  .picker:hover {
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 95%, transparent);
  }
  .picker:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
    outline-offset: 2px;
  }

  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }
  .title {
    font-size: 15px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    font-size: 12.5px;
  }

  .avatar {
    --m3e-avatar-size: 40px;
    flex: none;
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

  .play {
    display: flex;
    margin-top: 4px;
    width: 100%;
    --m3e-button-large-container-height: 64px;
  }

  .actions {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    justify-items: center;
    padding-top: 2px;
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(10px) scale(0.985);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .card {
      animation: none;
    }
  }
</style>
