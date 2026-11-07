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

/** Run `load` once, when the element first scrolls (nearly) into view. */
export function lazy(node: HTMLElement, load: () => void) {
  if (typeof IntersectionObserver === 'undefined') {
    load();
    return {};
  }
  const observer = new IntersectionObserver(
    (entries) => {
      if (entries.some((e) => e.isIntersecting)) {
        observer.disconnect();
        load();
      }
    },
    { rootMargin: '240px' },
  );
  observer.observe(node);
  return { destroy: () => observer.disconnect() };
}

/**
 * Move an element to the themed root so `position: fixed` covers the whole window.
 * (Panels use backdrop-filter, which would otherwise trap fixed children inside them.)
 */
export function portal(node: HTMLElement) {
  const root = document.querySelector('m3e-theme') ?? document.body;
  root.appendChild(node);
  return { destroy: () => node.remove() };
}
