<script lang="ts">
  import { onMount } from "svelte";

  type Item =
    | { label: string; action: () => void; disabled?: boolean }
    | { separator: true };

  type Props = { x: number; y: number; items: Item[]; onclose: () => void };
  let { x, y, items, onclose }: Props = $props();

  let menu: HTMLDivElement;
  let top = $state(0);
  let left = $state(0);

  onMount(() => {
    const rect = menu.getBoundingClientRect();
    top = y + rect.height > window.innerHeight ? Math.max(0, y - rect.height) : y;
    left = x + rect.width > window.innerWidth ? Math.max(0, x - rect.width) : x;

    function handleClick(e: MouseEvent) {
      if (!menu.contains(e.target as Node)) onclose();
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onclose();
    }
    window.addEventListener("mousedown", handleClick);
    window.addEventListener("keydown", handleKey);
    return () => {
      window.removeEventListener("mousedown", handleClick);
      window.removeEventListener("keydown", handleKey);
    };
  });
</script>

<div
  bind:this={menu}
  class="fixed z-50 w-56 rounded-lg border border-zinc-700 bg-zinc-900 shadow-xl py-1 text-sm"
  style="left: {left}px; top: {top}px;"
>
  {#each items as item}
    {#if "separator" in item}
      <div class="my-1 border-t border-zinc-700"></div>
    {:else}
      <button
        class="w-full text-left px-3 py-1.5 transition-colors text-xs leading-snug
          {item.disabled
            ? 'text-zinc-600 cursor-default'
            : 'text-zinc-200 hover:bg-zinc-700 cursor-pointer'}"
        disabled={item.disabled}
        onclick={() => { item.action(); onclose(); }}
      >
        {item.label}
      </button>
    {/if}
  {/each}
</div>
