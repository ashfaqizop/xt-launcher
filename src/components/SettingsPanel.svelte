<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import Panel from './Panel.svelte';
  import { nav } from '../lib/nav.svelte';
  import { resetSettings, settings } from '../lib/settings.svelte';
  import flame from '../assets/flame.png';
  import { GALLERY, refreshCustomFlag, setCustomWallpaper, wp } from '../lib/wallpaper.svelte';
  import { showToast } from '../lib/toast.svelte';
  import { engine } from '../lib/engine.svelte';
  import { autoMemoryMb, gib, loadSystem, maxGameMemoryGiB, system } from '../lib/system.svelte';
  import { checkForUpdates, type UpdateResult } from '../lib/updater';
  import { getVersion } from '@tauri-apps/api/app';
  import { openUrl } from '@tauri-apps/plugin-opener';

  type ValueEl = HTMLElement & { value: string };
  type CheckEl = HTMLElement & { checked: boolean };
  const val = (e: Event) => (e.currentTarget as ValueEl).value;
  const chk = (e: Event) => (e.currentTarget as CheckEl).checked;
  const num = (e: Event) => Number((e.currentTarget as ValueEl).value);

  const inTauri = '__TAURI_INTERNALS__' in window;

  let version = $state('0.1.0');
  let fileInput: HTMLInputElement;
  let update = $state<UpdateResult | null>(null);
  let checking = $state(false);
  let installing = $state(false);
  let percent = $state(0);

  // Java: "auto", a detected install's path, or "custom" (typed path)
  const javaChoice = $derived(
    settings.javaMode === 'auto'
      ? 'auto'
      : system.javas.some((j) => j.path === settings.javaPath)
        ? settings.javaPath
        : 'custom',
  );
  function onJava(e: Event) {
    const v = val(e);
    if (v === 'auto') settings.javaMode = 'auto';
    else if (v === 'custom') {
      settings.javaMode = 'custom';
      if (system.javas.some((j) => j.path === settings.javaPath)) settings.javaPath = '';
    } else {
      settings.javaMode = 'custom';
      settings.javaPath = v;
    }
  }

  // Graphics: only offer a choice when the PC has more than one card
  const discrete = $derived(system.gpus.filter((g) => !g.integrated));
  const integrated = $derived(system.gpus.filter((g) => g.integrated));
  const hasChoice = $derived(discrete.length > 0 && integrated.length > 0);

  onMount(() => {
    void loadSystem();
    void refreshCustomFlag();
    if (inTauri) void getVersion().then((v) => (version = v));
  });

  async function pickFile(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    try {
      await setCustomWallpaper(file);
    } catch {
      showToast("Couldn't use that image");
    }
  }

  async function checkUpdates() {
    checking = true;
    update = await checkForUpdates();
    checking = false;
  }

  async function install() {
    if (update?.kind !== 'available') return;
    installing = true;
    try {
      await update.install((p) => (percent = p));
    } catch {
      installing = false;
      showToast('Update failed');
    }
  }

  const open = (url: string) => (inTauri ? openUrl(url) : window.open(url, '_blank'));
  const ramGiB = $derived(settings.ramMb / 1024);
</script>

<Panel title="Settings" wide>
  <m3e-tabs>
    <m3e-tab for="tab-mc" selected={nav.tab === 'minecraft'} onclick={() => (nav.tab = 'minecraft')}>Minecraft</m3e-tab>
    <m3e-tab for="tab-launcher" selected={nav.tab === 'launcher'} onclick={() => (nav.tab = 'launcher')}>Launcher</m3e-tab>
    <m3e-tab for="tab-about" selected={nav.tab === 'about'} onclick={() => (nav.tab = 'about')}>About</m3e-tab>

    <!-- ================= Minecraft ================= -->
    <m3e-tab-panel id="tab-mc">
      <div class="tab">
        <h3>Game</h3>
        <div class="row">
          <span class="label">Game folder</span>
          <m3e-form-field variant="outlined" class="ctl">
            <input value={engine.dataDir || settings.gameFolder} readonly aria-label="Game folder" />
          </m3e-form-field>
        </div>
        <div class="row">
          <span class="label">Window size</span>
          <div class="ctl pair">
            <m3e-form-field variant="outlined">
              <input type="number" min="640" max="7680" bind:value={settings.winW} aria-label="Width" />
            </m3e-form-field>
            <span class="muted">×</span>
            <m3e-form-field variant="outlined">
              <input type="number" min="480" max="4320" bind:value={settings.winH} aria-label="Height" />
            </m3e-form-field>
          </div>
        </div>
        <div class="row">
          <span class="label">Fullscreen</span>
          <m3e-switch class="sw" checked={settings.fullscreen} onchange={(e: Event) => (settings.fullscreen = chk(e))}></m3e-switch>
        </div>

        <h3>Java and memory</h3>
        <div class="row">
          <span class="label">Java</span>
          <m3e-form-field variant="outlined" class="ctl">
            <m3e-select value={javaChoice} aria-label="Java runtime" onchange={onJava}>
              <m3e-option value="auto">Automatic (recommended)</m3e-option>
              {#each system.javas as j (j.path)}
                <m3e-option value={j.path}>Java {j.major} · {j.vendor}{j.managed ? ' (XT)' : ''}</m3e-option>
              {/each}
              <m3e-option value="custom">Other location…</m3e-option>
            </m3e-select>
          </m3e-form-field>
        </div>
        {#if javaChoice === 'custom'}
          <div class="row">
            <span class="label">Java path</span>
            <m3e-form-field variant="outlined" class="ctl">
              <input bind:value={settings.javaPath} placeholder="C:Program FilesJavajdk-21injava.exe" spellcheck="false" aria-label="Java path" />
            </m3e-form-field>
          </div>
        {:else if javaChoice !== 'auto'}
          <div class="row">
            <span class="label"></span>
            <span class="muted note">{settings.javaPath}</span>
          </div>
        {:else}
          <div class="row">
            <span class="label"></span>
            <span class="muted note">Each Minecraft version gets the Java it needs, downloaded automatically.</span>
          </div>
        {/if}

        <div class="row">
          <span class="label">Memory</span>
          <label class="auto">
            <m3e-switch checked={settings.ramAuto} onchange={(e: Event) => (settings.ramAuto = chk(e))}></m3e-switch>
            <span>Automatic</span>
          </label>
        </div>
        <div class="row">
          <span class="label"></span>
          {#if settings.ramAuto}
            <span class="muted note">
              {gib(autoMemoryMb())} GiB{system.totalMemoryMb ? ` of ${gib(system.totalMemoryMb)} GiB installed` : ''}, chosen for this PC
            </span>
          {:else}
            <div class="ctl mem">
              <m3e-slider min="1" max={maxGameMemoryGiB()} step="0.5" labelled class="slider" aria-label="Memory in GiB">
                <m3e-slider-thumb value={Math.min(ramGiB, maxGameMemoryGiB())} oninput={(e: Event) => (settings.ramMb = Math.round(num(e) * 1024))}></m3e-slider-thumb>
              </m3e-slider>
              <span class="amount">{gib(settings.ramMb)} GiB</span>
            </div>
          {/if}
        </div>

        <div class="row">
          <span class="label">Graphics card</span>
          {#if hasChoice}
            <m3e-form-field variant="outlined" class="ctl">
              <m3e-select value={settings.gpu} aria-label="Graphics card" onchange={(e: Event) => (settings.gpu = val(e) as typeof settings.gpu)}>
                <m3e-option value="default">Let Windows decide</m3e-option>
                {#each discrete as g (g.name)}
                  <m3e-option value="high">{g.name}</m3e-option>
                {/each}
                {#each integrated as g (g.name)}
                  <m3e-option value="low">{g.name}</m3e-option>
                {/each}
              </m3e-select>
            </m3e-form-field>
          {:else}
            <span class="muted note">{system.gpus[0]?.name ?? 'Detecting…'}</span>
          {/if}
        </div>

        <h3>Version list</h3>
        <div class="row">
          <span class="label">Show snapshots</span>
          <m3e-switch class="sw" checked={settings.showSnapshots} onchange={(e: Event) => (settings.showSnapshots = chk(e))}></m3e-switch>
        </div>
        <div class="row">
          <span class="label">Show old versions (Beta / Alpha)</span>
          <m3e-switch class="sw" checked={settings.showOld} onchange={(e: Event) => (settings.showOld = chk(e))}></m3e-switch>
        </div>
      </div>
    </m3e-tab-panel>

    <!-- ================= Launcher ================= -->
    <m3e-tab-panel id="tab-launcher">
      <div class="tab">
        <h3>Appearance</h3>
        <div class="row">
          <span class="label">Theme</span>
          <m3e-segmented-button class="ctl seg">
            <m3e-button-segment value="system" checked={settings.themeMode === 'system'} onclick={() => (settings.themeMode = 'system')}>System</m3e-button-segment>
            <m3e-button-segment value="light" checked={settings.themeMode === 'light'} onclick={() => (settings.themeMode = 'light')}>Light</m3e-button-segment>
            <m3e-button-segment value="dark" checked={settings.themeMode === 'dark'} onclick={() => (settings.themeMode = 'dark')}>Dark</m3e-button-segment>
          </m3e-segmented-button>
        </div>
        <div class="row">
          <span class="label">Colours from wallpaper</span>
          <m3e-switch class="sw" checked={settings.dynamicColor} onchange={(e: Event) => (settings.dynamicColor = chk(e))}></m3e-switch>
        </div>

        <div class="row top">
          <span class="label">Wallpaper</span>
          <div class="ctl">
            <div class="gallery">
              {#each GALLERY as g (g.id)}
                <button class="tile" class:on={settings.wallpaper === g.id} onclick={() => (settings.wallpaper = g.id)} aria-label={g.name} aria-pressed={settings.wallpaper === g.id}>
                  <img src={g.url} alt="" loading="lazy" decoding="async" draggable="false" />
                  <span class="cap">{g.name}</span>
                </button>
              {/each}
              <button class="tile own" class:on={settings.wallpaper === 'custom'} onclick={() => (wp.hasCustom ? (settings.wallpaper = 'custom') : fileInput.click())} aria-pressed={settings.wallpaper === 'custom'}>
                <Icon name="wallpaper" size={28} />
                <span class="cap">{wp.hasCustom ? 'Your image' : 'Add image'}</span>
              </button>
            </div>
            <div class="gal-actions">
              <m3e-button variant="text" size="small" onclick={() => fileInput.click()}>Choose image…</m3e-button>
              <span class="muted credit">Minecraft artwork © Mojang AB / Microsoft, from minecraft.net</span>
            </div>
            <input bind:this={fileInput} type="file" accept="image/*" hidden onchange={pickFile} />
          </div>
        </div>

        <div class="row">
          <span class="label">Blur</span>
          <div class="ctl mem">
            <m3e-slider min="0" max="30" step="1" labelled class="slider" aria-label="Wallpaper blur">
              <m3e-slider-thumb value={settings.blur} oninput={(e: Event) => (settings.blur = num(e))}></m3e-slider-thumb>
            </m3e-slider>
          </div>
        </div>
        <div class="row">
          <span class="label">Dim</span>
          <div class="ctl mem">
            <m3e-slider min="0" max="80" step="1" labelled class="slider" aria-label="Wallpaper dim">
              <m3e-slider-thumb value={settings.dim} oninput={(e: Event) => (settings.dim = num(e))}></m3e-slider-thumb>
            </m3e-slider>
          </div>
        </div>

        <h3>Behaviour</h3>
        <div class="row">
          <span class="label">After starting Minecraft</span>
          <m3e-form-field variant="outlined" class="ctl">
            <m3e-select value={settings.afterLaunch} aria-label="After starting Minecraft" onchange={(e: Event) => (settings.afterLaunch = val(e) as typeof settings.afterLaunch)}>
              <m3e-option value="tray">Minimize launcher</m3e-option>
              <m3e-option value="close">Close launcher</m3e-option>
              <m3e-option value="keep">Keep open</m3e-option>
            </m3e-select>
          </m3e-form-field>
        </div>
        <div class="row">
          <span class="label">Scan for crashes</span>
          <m3e-switch class="sw" checked={settings.crashScan} onchange={(e: Event) => (settings.crashScan = chk(e))}></m3e-switch>
        </div>
        <div class="row">
          <span class="label">Notify on new release</span>
          <m3e-switch class="sw" checked={settings.alertRelease} onchange={(e: Event) => (settings.alertRelease = chk(e))}></m3e-switch>
        </div>
        <div class="row">
          <span class="label">Notify on new snapshot</span>
          <m3e-switch class="sw" checked={settings.alertSnapshot} onchange={(e: Event) => (settings.alertSnapshot = chk(e))}></m3e-switch>
        </div>
        <div class="row">
          <span class="label">Language</span>
          <m3e-form-field variant="outlined" class="ctl">
            <m3e-select value="en" disabled aria-label="Language">
              <m3e-option value="en">English</m3e-option>
            </m3e-select>
          </m3e-form-field>
        </div>
      </div>
    </m3e-tab-panel>

    <!-- ================= About ================= -->
    <m3e-tab-panel id="tab-about">
      <div class="tab about">
        <div class="hero">
          <img src={flame} alt="" height="46" draggable="false" />
          <div>
            <div class="name">XT Launcher</div>
            <div class="muted">Version {version}</div>
          </div>
        </div>
        <p class="muted blurb">A clean, fast Minecraft Java launcher. No telemetry, no ads.</p>

        <div class="update">
          {#if installing}
            <m3e-circular-progress-indicator variant="wavy" value={percent} max="100" class="prog"></m3e-circular-progress-indicator>
            <span>Installing update… {Math.floor(percent)}%</span>
          {:else if update?.kind === 'available'}
            <span>Version {update.version} is available</span>
            <m3e-button variant="filled" onclick={install}><span slot="icon"><Icon name="download" size={20} /></span>Install and restart</m3e-button>
          {:else}
            <m3e-button variant="tonal" disabled={checking} onclick={checkUpdates}>
              <span slot="icon"><Icon name="refresh" size={20} /></span>{checking ? 'Checking…' : 'Check for updates'}
            </m3e-button>
            {#if update?.kind === 'latest'}<span class="muted">You're up to date</span>{/if}
            {#if update?.kind === 'error'}<span class="muted">Couldn't reach GitHub</span>{/if}
          {/if}
        </div>

        <div class="links">
          <m3e-button variant="text" onclick={() => open('https://github.com/ashfaqizop/xt-launcher')}>
            <span slot="icon"><Icon name="open" size={18} /></span>Source on GitHub
          </m3e-button>
          <m3e-button variant="text" onclick={() => open('https://ely.by')}>
            <span slot="icon"><Icon name="open" size={18} /></span>Ely.by
          </m3e-button>
        </div>

        <p class="legal muted">
          XT Launcher is not an official Minecraft product and is not approved by or associated with Mojang or Microsoft.
          Minecraft artwork used as wallpapers is © Mojang AB / Microsoft and comes from minecraft.net.
        </p>
      </div>
    </m3e-tab-panel>
  </m3e-tabs>

  {#snippet footer()}
    <m3e-button variant="text" onclick={resetSettings}>Reset to defaults</m3e-button>
  {/snippet}
</Panel>

<style>
  .tab {
    padding: 4px 4px 8px;
  }
  h3 {
    margin: 18px 0 6px;
    font-size: 13px;
    font-weight: 650;
    letter-spacing: 0.02em;
    color: var(--md-sys-color-primary, #c23a24);
  }
  .row {
    display: grid;
    grid-template-columns: 200px 1fr;
    align-items: center;
    gap: 16px;
    min-height: 46px;
  }
  .row.top {
    align-items: start;
    padding-top: 10px;
  }
  .label {
    font-size: 14px;
    font-weight: 500;
  }
  .ctl {
    min-width: 0;
    width: 100%;
    max-width: 360px;
  }
  .sw {
    justify-self: start;
  }
  .pair {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .pair m3e-form-field {
    flex: 1;
  }
  .mem {
    max-width: 420px;
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .auto {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 14px;
    cursor: pointer;
  }
  .slider {
    flex: 1;
  }
  .note {
    font-size: 12.5px;
    line-height: 1.45;
    word-break: break-all;
  }
  .amount {
    min-width: 54px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .seg {
    justify-self: start;
    width: auto;
  }

  .gallery {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(116px, 1fr));
    gap: 8px;
  }
  .tile {
    all: unset;
    box-sizing: border-box;
    position: relative;
    aspect-ratio: 16 / 10;
    border-radius: 14px;
    overflow: hidden;
    cursor: pointer;
    outline: 2px solid transparent;
    outline-offset: 2px;
    transition: outline-color 120ms;
  }
  .tile img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .tile.on {
    outline-color: var(--md-sys-color-primary, #c23a24);
  }
  .tile:focus-visible {
    outline-color: var(--md-sys-color-tertiary, #e6c36a);
  }
  .cap {
    position: absolute;
    inset: auto 0 0 0;
    padding: 14px 8px 5px;
    font-size: 11.5px;
    font-weight: 600;
    color: #fff;
    background: linear-gradient(transparent, rgb(0 0 0 / 0.7));
  }
  .tile.own {
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 4px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 80%, transparent);
  }
  .tile.own .cap {
    position: static;
    background: none;
    padding: 0;
    color: inherit;
  }
  .gal-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: 6px;
  }
  .credit {
    font-size: 11.5px;
  }

  .about {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding-top: 12px;
  }
  .hero {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .name {
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .blurb {
    margin: 0;
  }
  .update {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .prog {
    --m3e-circular-wavy-progress-indicator-diameter: 36px;
  }
  .links {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  .legal {
    margin: 4px 0 0;
    font-size: 12px;
    line-height: 1.5;
  }
</style>
