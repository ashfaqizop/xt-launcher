<script lang="ts">
  import Icon from '../lib/Icon.svelte';
  import Panel from './Panel.svelte';
  import { PROFILES, data, profileSubtitle } from '../lib/data.svelte';
</script>

<Panel title="Profiles">
  <ul class="list">
    {#each PROFILES as p (p.id)}
      <li class:active={p.id === data.profileId}>
        <button class="row" onclick={() => (data.profileId = p.id)}>
          <span class="chip-icon" class:xt={p.kind !== 'custom'}><Icon name="rocket" size={20} /></span>
          <span class="text">
            <span class="title">{p.name}</span>
            <span class="sub muted">{profileSubtitle(p)}</span>
          </span>
          {#if p.kind === 'xt'}
            <span class="tag">Optimized</span>
          {:else if p.kind === 'xt-plus'}
            <span class="tag">Optimized+</span>
          {/if}
          {#if p.id === data.profileId}<Icon name="check" size={20} />{/if}
        </button>
      </li>
    {/each}
  </ul>
  <p class="hint muted">Creating, editing and duplicating profiles arrives with the game engine.</p>
</Panel>

<style>
  .list {
    list-style: none;
    margin: 4px 0 8px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  li {
    border-radius: 20px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest, #383434) 55%, transparent);
  }
  li.active {
    background: color-mix(in srgb, var(--md-sys-color-primary-container, #8f2410) 70%, transparent);
  }
  .row {
    all: unset;
    box-sizing: border-box;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px 10px 10px;
    cursor: pointer;
  }
  .row:focus-visible {
    outline: 2px solid var(--md-sys-color-primary, #c23a24);
    border-radius: 20px;
  }
  .chip-icon {
    width: 40px;
    height: 40px;
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
  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .title {
    font-weight: 600;
  }
  .sub {
    font-size: 12.5px;
  }
  .tag {
    font-size: 11.5px;
    font-weight: 600;
    padding: 3px 9px;
    border-radius: 999px;
    background: var(--md-sys-color-tertiary-container, #5b4a1f);
    color: var(--md-sys-color-on-tertiary-container, #ffe08c);
  }
  .hint {
    margin: 12px 4px 0;
    font-size: 12.5px;
  }
</style>
