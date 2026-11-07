// Drag to reorder a list. Pointer-based (so it behaves the same with mouse, pen and touch),
// animates the neighbours out of the way, and works from the keyboard with Alt+Up/Down.
//
//   <ul use:sortable={{ onMove: (from, to) => ... }}>
//     <li><button data-handle aria-label="Reorder">...</button> ...</li>
//   </ul>

export interface SortableOptions {
  /** Called once when the item is dropped on a new position */
  onMove: (from: number, to: number) => void;
  /** Selector for the grab handle inside an item (default: [data-handle]) */
  handle?: string;
  /** Selector for the items, direct children by default */
  item?: string;
}

interface Drag {
  el: HTMLElement;
  handle: HTMLElement;
  items: HTMLElement[];
  rects: DOMRect[];
  index: number;
  target: number;
  startY: number;
  step: number;
  pointerId: number;
}

export function sortable(node: HTMLElement, initial: SortableOptions) {
  let options = initial;
  let drag: Drag | null = null;

  const items = () => Array.from(node.querySelectorAll<HTMLElement>(options.item ?? ':scope > *'));

  function handleOf(target: EventTarget | null): HTMLElement | null {
    if (!(target instanceof Element)) return null;
    const h = target.closest<HTMLElement>(options.handle ?? '[data-handle]');
    return h && node.contains(h) ? h : null;
  }

  function start(e: PointerEvent) {
    if (e.button !== 0 || drag) return;
    const handle = handleOf(e.target);
    if (!handle) return;
    const list = items();
    const el = list.find((item) => item.contains(handle));
    if (!el) return;

    e.preventDefault();
    handle.setPointerCapture(e.pointerId);
    const rects = list.map((item) => item.getBoundingClientRect());
    const index = list.indexOf(el);
    const gap = list.length > 1 ? Math.max(0, rects[1].top - rects[0].bottom) : 0;
    drag = {
      el,
      handle,
      items: list,
      rects,
      index,
      target: index,
      startY: e.clientY,
      step: rects[index].height + gap,
      pointerId: e.pointerId,
    };

    node.classList.add('sorting');
    el.classList.add('dragging');
    for (const item of list) {
      if (item !== el) item.style.transition = 'transform 180ms cubic-bezier(0.2, 0, 0, 1)';
    }
  }

  function move(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.pointerId) return;
    const { rects, index, items: list, el } = drag;

    // Keep the dragged row inside the list
    const first = rects[0].top;
    const last = rects[rects.length - 1].bottom - rects[index].height;
    const raw = e.clientY - drag.startY;
    const dy = Math.min(last - rects[index].top, Math.max(first - rects[index].top, raw));
    el.style.transform = `translateY(${dy}px)`;

    // Where the row's centre sits among the others decides the drop position
    const centre = rects[index].top + rects[index].height / 2 + dy;
    let target = 0;
    list.forEach((_, i) => {
      if (i === index) return;
      const mid = rects[i].top + rects[i].height / 2;
      if (mid < centre) target++;
    });
    drag.target = target;

    list.forEach((item, i) => {
      if (i === index) return;
      let shift = 0;
      if (index < target && i > index && i <= target) shift = -drag!.step;
      else if (index > target && i >= target && i < index) shift = drag!.step;
      item.style.transform = shift ? `translateY(${shift}px)` : '';
    });
  }

  function finish(e: PointerEvent, cancelled: boolean) {
    if (!drag || e.pointerId !== drag.pointerId) return;
    const { from, to, list, el, handle } = { from: drag.index, to: drag.target, list: drag.items, el: drag.el, handle: drag.handle };
    drag = null;
    if (handle.hasPointerCapture(e.pointerId)) handle.releasePointerCapture(e.pointerId);

    const reset = () => {
      for (const item of list) {
        item.style.transform = '';
        item.style.transition = '';
      }
      el.classList.remove('dragging');
      node.classList.remove('sorting');
    };

    if (cancelled || from === to) {
      el.style.transition = 'transform 180ms cubic-bezier(0.2, 0, 0, 1)';
      requestAnimationFrame(reset);
      return;
    }
    // Reorder the data first; the DOM follows before the next paint, then the offsets go away
    options.onMove(from, to);
    requestAnimationFrame(() => {
      for (const item of list) item.style.transition = 'none';
      reset();
    });
  }

  function key(e: KeyboardEvent) {
    if (!e.altKey || (e.key !== 'ArrowUp' && e.key !== 'ArrowDown')) return;
    const handle = handleOf(e.target);
    if (!handle) return;
    const list = items();
    const index = list.findIndex((item) => item.contains(handle));
    const to = index + (e.key === 'ArrowUp' ? -1 : 1);
    if (index < 0 || to < 0 || to >= list.length) return;
    e.preventDefault();
    options.onMove(index, to);
    // Keep focus on the same row's handle after it moved
    requestAnimationFrame(() => {
      const moved = items()[to];
      moved?.querySelector<HTMLElement>(options.handle ?? '[data-handle]')?.focus();
    });
  }

  const onUp = (e: PointerEvent) => finish(e, false);
  const onCancel = (e: PointerEvent) => finish(e, true);

  node.addEventListener('pointerdown', start);
  node.addEventListener('pointermove', move);
  node.addEventListener('pointerup', onUp);
  node.addEventListener('pointercancel', onCancel);
  node.addEventListener('keydown', key);

  return {
    update(next: SortableOptions) {
      options = next;
    },
    destroy() {
      node.removeEventListener('pointerdown', start);
      node.removeEventListener('pointermove', move);
      node.removeEventListener('pointerup', onUp);
      node.removeEventListener('pointercancel', onCancel);
      node.removeEventListener('keydown', key);
    },
  };
}
