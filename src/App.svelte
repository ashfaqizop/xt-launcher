<script lang="ts">
  import Titlebar from './components/Titlebar.svelte';
  import HomeCard from './components/HomeCard.svelte';
  import SettingsPanel from './components/SettingsPanel.svelte';
  import AccountsPanel from './components/AccountsPanel.svelte';
  import InstancesPanel from './components/InstancesPanel.svelte';
  import LogsPanel from './components/LogsPanel.svelte';
  import { initEngine } from './lib/engine.svelte';
  import { nav } from './lib/nav.svelte';
  import { settings } from './lib/settings.svelte';
  import { BRAND_SEED, applyWallpaper, wp } from './lib/wallpaper.svelte';
  import { toast } from './lib/toast.svelte';

  void initEngine();

  // Resolve the chosen wallpaper (and its seed colour) whenever the setting changes.
  $effect(() => {
    void applyWallpaper(settings.wallpaper);
  });

  const seed = $derived(settings.dynamicColor && wp.seed ? wp.seed : BRAND_SEED);
  const scheme = $derived(settings.themeMode === 'system' ? 'auto' : settings.themeMode);
</script>

<m3e-theme color={seed} {scheme} motion="expressive" density="-2" strong-focus>
  <div class="stage">
    <div
      class="wallpaper"
      style:background-image="url({wp.url})"
      style:--blur="{settings.blur}px"
    ></div>
    <div class="dim" style:opacity={settings.dim / 100}></div>

    <Titlebar />

    <main>
      {#if nav.view === 'home'}
        <HomeCard />
      {:else if nav.view === 'settings'}
        <SettingsPanel />
      {:else if nav.view === 'accounts'}
        <AccountsPanel />
      {:else if nav.view === 'instances'}
        <InstancesPanel />
      {:else if nav.view === 'logs'}
        <LogsPanel />
      {/if}
    </main>

    {#if toast.text}
      {#key toast.id}
        <div class="toast" role="status">{toast.text}</div>
      {/key}
    {/if}
  </div>
</m3e-theme>

<style>
  m3e-theme {
    display: block;
    height: 100%;
  }
  .stage {
    position: relative;
    height: 100vh;
    overflow: hidden;
    background: var(--md-sys-color-surface, #100f0f);
    color: var(--md-sys-color-on-surface, #ece0dd);
  }
  .wallpaper {
    position: absolute;
    inset: calc(var(--blur) * -2);
    background-size: cover;
    background-position: center;
    filter: blur(var(--blur));
    transition: filter 160ms;
  }
  .dim {
    position: absolute;
    inset: 0;
    background: #000;
    pointer-events: none;
  }
  main {
    position: absolute;
    inset: 44px 0 0 0;
    display: grid;
    place-items: center;
    padding: 0 24px 24px;
  }
  .toast {
    position: absolute;
    left: 50%;
    bottom: 28px;
    transform: translateX(-50%);
    max-width: min(520px, calc(100vw - 48px));
    padding: 12px 20px;
    border-radius: 16px;
    font-size: 14px;
    font-weight: 500;
    background: var(--md-sys-color-inverse-surface, #ece0dd);
    color: var(--md-sys-color-inverse-on-surface, #322f2f);
    box-shadow: 0 8px 28px rgb(0 0 0 / 0.4);
    animation: toast 220ms cubic-bezier(0.2, 0, 0, 1);
    z-index: 30;
  }
  @keyframes toast {
    from {
      opacity: 0;
      transform: translate(-50%, 8px);
    }
  }
</style>
