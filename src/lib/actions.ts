// Svelte actions for M3E custom elements.

/**
 * Set an M3E element's `value` property after it is connected (and upgraded).
 * Assigning it in the template can happen before the custom element upgrades,
 * which shadows Lit's accessor so later updates are ignored.
 */
export function selectValue(node: HTMLElement & { value?: unknown }, value: string) {
  const apply = (v: string) => {
    if (node.value !== v) node.value = v;
  };
  if (customElements.get(node.localName)) customElements.upgrade(node);
  apply(value);
  return { update: apply };
}
