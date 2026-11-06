<script lang="ts">
  import { icons, type IconName } from './icons';

  let { name, size = 24 }: { name: IconName; size?: number } = $props();
</script>

<span class="icon" style:--s="{size}px" aria-hidden="true">{@html icons[name]}</span>

<style>
  .icon {
    display: inline-grid;
    place-items: center;
    width: var(--s);
    height: var(--s);
    flex: none;
    line-height: 0;
  }
  .icon :global(svg) {
    width: var(--s);
    height: var(--s);
    fill: currentColor;
  }
</style>
