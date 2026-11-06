<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import { getCurrentWindow, type Window } from '@tauri-apps/api/window';

  const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  let win: Window | null = null;
  let maximized = $state(false);

  onMount(() => {
    if (!inTauri) return;
    win = getCurrentWindow();
    const sync = async () => (maximized = await win!.isMaximized());
    void sync();
    let unlisten: (() => void) | undefined;
    void win.onResized(sync).then((u) => (unlisten = u));
    return () => unlisten?.();
  });
</script>

<header class="bar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <span class="dot"></span>
    <span data-tauri-drag-region>XT Launcher</span>
  </div>

  <div class="caption">
    <m3e-icon-button size="small" aria-label="Minimize" onclick={() => win?.minimize()}>
      <Icon name="minimize" size={20} />
    </m3e-icon-button>
    <m3e-icon-button size="small" aria-label={maximized ? 'Restore' : 'Maximize'} onclick={() => win?.toggleMaximize()}>
      <Icon name={maximized ? 'restore' : 'maximize'} size={18} />
    </m3e-icon-button>
    <m3e-icon-button size="small" aria-label="Close" class="close" onclick={() => win?.close()}>
      <Icon name="close" size={20} />
    </m3e-icon-button>
  </div>
</header>

<style>
  .bar {
    position: absolute;
    inset: 0 0 auto 0;
    height: 44px;
    z-index: 20;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px 0 16px;
    color: var(--md-sys-color-on-surface, #ece0dd);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.01em;
    opacity: 0.85;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--md-sys-color-primary, #c23a24);
  }
  .caption {
    display: flex;
    gap: 2px;
  }
  .close:hover {
    --m3e-icon-button-hover-color: var(--md-sys-color-error, #ffb4ab);
  }
</style>
