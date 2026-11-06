<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import { go } from '../lib/nav.svelte';

  let {
    title,
    wide = false,
    children,
    footer,
  }: { title: string; wide?: boolean; children: Snippet; footer?: Snippet } = $props();
</script>

<section class="panel glass" class:wide>
  <header>
    <m3e-icon-button aria-label="Back to home" onclick={() => go('home')}>
      <Icon name="back" />
    </m3e-icon-button>
    <h2>{title}</h2>
  </header>
  <div class="body">
    {@render children()}
  </div>
  {#if footer}
    <footer>{@render footer()}</footer>
  {/if}
</section>

<style>
  .panel {
    width: min(560px, calc(100vw - 48px));
    max-height: calc(100vh - 92px);
    display: flex;
    flex-direction: column;
    animation: rise 260ms cubic-bezier(0.2, 0, 0, 1);
  }
  .panel.wide {
    width: min(760px, calc(100vw - 48px));
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 14px 16px 6px 12px;
  }
  h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 650;
    letter-spacing: -0.005em;
  }
  .body {
    padding: 8px 20px 20px;
    overflow-y: auto;
    min-height: 0;
  }
  footer {
    padding: 12px 20px 16px;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    border-top: 1px solid color-mix(in srgb, var(--md-sys-color-outline-variant, #504442) 50%, transparent);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(10px) scale(0.985);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .panel {
      animation: none;
    }
  }
</style>
