<script lang="ts">
  export type MenuItem =
    | { label: string; action: () => void; shortcut?: string; disabled?: boolean }
    | { separator: true };

  export type MenuDef = { label: string; items: MenuItem[] };

  type Props = { menus: MenuDef[] };
  let { menus }: Props = $props();

  let openIdx = $state<number | null>(null);
  // torn: menuIdx → { x, y }
  let torn = $state<Map<number, { x: number; y: number }>>(new Map());
  let drag = $state<{ idx: number; ox: number; oy: number; mx: number; my: number } | null>(null);

  function toggle(idx: number) {
    if (torn.has(idx)) return;
    openIdx = openIdx === idx ? null : idx;
  }

  function tearoff(idx: number, ref: HTMLElement) {
    const rect = ref.getBoundingClientRect();
    const m = new Map(torn);
    m.set(idx, { x: rect.left, y: rect.bottom + 6 });
    torn = m;
    openIdx = null;
  }

  function closeTorn(idx: number) {
    const m = new Map(torn);
    m.delete(idx);
    torn = m;
  }

  function startDrag(idx: number, e: MouseEvent) {
    const pos = torn.get(idx)!;
    drag = { idx, ox: pos.x, oy: pos.y, mx: e.clientX, my: e.clientY };
    e.preventDefault();
  }

  function onMouseMove(e: MouseEvent) {
    if (!drag) return;
    const m = new Map(torn);
    m.set(drag.idx, { x: drag.ox + e.clientX - drag.mx, y: drag.oy + e.clientY - drag.my });
    torn = m;
  }

  function onMouseUp() { drag = null; }

  function onWindowClick(e: MouseEvent) {
    if (openIdx === null) return;
    if (!(e.target as HTMLElement).closest('[data-menubar]')) openIdx = null;
  }
</script>

<svelte:window onmousemove={onMouseMove} onmouseup={onMouseUp} onclick={onWindowClick} />

<div class="flex items-center" data-menubar>
  {#each menus as menu, idx}
    {@const isTorn = torn.has(idx)}
    <div class="relative">
      <button
        data-menu-trigger={idx}
        onclick={(e) => { toggle(idx); }}
        class="px-3 py-1 text-sm rounded transition-colors cursor-pointer
          {openIdx === idx ? 'bg-zinc-700 text-white' : isTorn ? 'text-zinc-600' : 'text-zinc-300 hover:text-white hover:bg-zinc-800'}"
      >{menu.label}</button>

      {#if openIdx === idx}
        {@const triggerEl = document.querySelector(`[data-menu-trigger="${idx}"]`) as HTMLElement}
        <div class="absolute top-full left-0 mt-1 bg-zinc-800 border border-zinc-700 rounded shadow-xl z-50 min-w-44 py-1 select-none">
          <!-- Tearoff-rad -->
          <button
            onclick={() => tearoff(idx, triggerEl)}
            class="w-full px-3 py-0.5 text-center border-b border-dashed border-zinc-600 text-zinc-500 hover:text-zinc-300 hover:bg-zinc-700/40 cursor-grab transition-colors text-xs tracking-widest"
          >· · · · · · · · · · · ·</button>

          {#each menu.items as item}
            {#if 'separator' in item}
              <hr class="my-1 border-zinc-700" />
            {:else}
              <button
                onclick={() => { if (!item.disabled) { item.action(); openIdx = null; } }}
                disabled={item.disabled}
                class="w-full text-left px-3 py-1.5 text-sm flex items-center justify-between gap-8
                  {item.disabled ? 'text-zinc-600 cursor-default' : 'text-zinc-300 hover:bg-zinc-700 hover:text-white cursor-pointer'}"
              >
                <span>{item.label}</span>
                {#if item.shortcut}<span class="text-zinc-500 text-xs">{item.shortcut}</span>{/if}
              </button>
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</div>

<!-- Flytande, utrivna menyer -->
{#each [...torn] as [idx, pos]}
  {@const menu = menus[idx]}
  <div
    class="fixed z-50 bg-zinc-800 border border-zinc-700 rounded shadow-2xl min-w-44 select-none"
    style="left:{pos.x}px; top:{pos.y}px;"
  >
    <!-- Draghandtag -->
    <div
      class="flex items-center justify-between px-2 py-1 border-b border-dashed border-zinc-600 cursor-grab"
      onmousedown={(e) => startDrag(idx, e)}
    >
      <span class="text-xs text-zinc-400">{menu.label}</span>
      <button
        onclick={() => closeTorn(idx)}
        class="text-zinc-500 hover:text-white transition-colors leading-none ml-4 cursor-pointer"
      >×</button>
    </div>
    <div class="py-1">
      {#each menu.items as item}
        {#if 'separator' in item}
          <hr class="my-1 border-zinc-700" />
        {:else}
          <button
            onclick={() => { if (!item.disabled) item.action(); }}
            disabled={item.disabled}
            class="w-full text-left px-3 py-1.5 text-sm flex items-center justify-between gap-8
              {item.disabled ? 'text-zinc-600 cursor-default' : 'text-zinc-300 hover:bg-zinc-700 hover:text-white cursor-pointer'}"
          >
            <span>{item.label}</span>
            {#if item.shortcut}<span class="text-zinc-500 text-xs">{item.shortcut}</span>{/if}
          </button>
        {/if}
      {/each}
    </div>
  </div>
{/each}
