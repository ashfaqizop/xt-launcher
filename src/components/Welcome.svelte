<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app';
  import { onMount } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import { inTauri } from '../lib/engine.svelte';
  import { gib, lowEndMemoryMb, system } from '../lib/system.svelte';
  import { go } from '../lib/nav.svelte';
  import { createLowEndProfile, dismissLowEnd, dismissWhatsNew, initWelcome, welcome } from '../lib/welcome.svelte';

  onMount(() => {
    if (!inTauri) return;
    // Let the main window settle before anything pops up over it
    const timer = setTimeout(() => {
      void getVersion().then((v) => initWelcome(v));
    }, 900);
    return () => clearTimeout(timer);
  });

  /** Escape, then turn **bold** into <strong>. */
  function inline(text: string): string {
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  }

  function createAndClose() {
    createLowEndProfile();
    go('home');
  }
</script>

<m3e-dialog open={!!welcome.notes} onclosed={dismissWhatsNew} class="notes">
  <span slot="header">What's new in XT Launcher</span>
  <div class="content">
    {#each welcome.notes ?? [] as release (release.version)}
      <section>
        <h4>Version {release.version}</h4>
        {#each release.groups as group (group.title)}
          <h5>{group.title}</h5>
          <ul>
            {#each group.items as item (item)}
              <li>{@html inline(item)}</li>
            {/each}
          </ul>
        {/each}
      </section>
    {/each}
  </div>
  <div slot="actions">
    <m3e-button variant="filled" onclick={dismissWhatsNew}>Got it</m3e-button>
  </div>
</m3e-dialog>

<m3e-dialog open={welcome.lowEnd} onclosed={dismissLowEnd}>
  <span slot="header">Set up for a smaller PC</span>
  <div class="content">
    <div class="hero">
      <span class="ic"><Icon name="memory" size={26} /></span>
      <p>
        This PC has <strong>{gib(system.totalMemoryMb)} GiB</strong> of memory. Minecraft runs well on it with
        <strong>XT Optimized</strong>: lighter mods, a smoother garbage collector, and a memory limit of
        <strong>{gib(lowEndMemoryMb())} GiB</strong> so Windows and your other apps keep their room.
      </p>
    </div>
    <p class="small">You can change the memory limit any time in the profile's settings.</p>
  </div>
  <div slot="actions" class="acts">
    <m3e-button variant="text" onclick={dismissLowEnd}>Not now</m3e-button>
    <m3e-button variant="filled" onclick={createAndClose}>Create XT Optimized</m3e-button>
  </div>
</m3e-dialog>

<style>
  .content {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 52vh;
    overflow-y: auto;
    padding-right: 6px;
  }
  section + section {
    margin-top: 12px;
  }
  h4 {
    margin: 0 0 2px;
    font-size: 15px;
    font-weight: 700;
  }
  h5 {
    margin: 10px 0 2px;
    font-size: 12px;
    font-weight: 650;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--md-sys-color-primary, #c23a24);
  }
  ul {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 14px;
    line-height: 1.45;
  }
  .hero {
    display: flex;
    gap: 14px;
    align-items: flex-start;
  }
  .hero p {
    margin: 0;
    font-size: 14.5px;
    line-height: 1.5;
  }
  .ic {
    width: 48px;
    height: 48px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 16px;
    background: var(--md-sys-color-primary-container, #8f2410);
    color: var(--md-sys-color-on-primary-container, #ffdad4);
  }
  .small {
    margin: 0;
    font-size: 12.5px;
    color: var(--md-sys-color-on-surface-variant, #bfb2ae);
  }
  .acts {
    display: flex;
    gap: 8px;
  }
</style>
