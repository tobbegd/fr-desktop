<script lang="ts">
  import { onMount } from "svelte";
  import { debug } from "$lib/debug.svelte";

  let height = $state(200);
  let dragging = false;
  let startY = 0;
  let startHeight = 0;

  onMount(() => {
    debug.log("Debug-konsoll startad");
  });

  function onDragStart(e: MouseEvent) {
    dragging = true;
    startY = e.clientY;
    startHeight = height;
    e.preventDefault();
  }

  function onMouseMove(e: MouseEvent) {
    if (!dragging) return;
    const delta = startY - e.clientY;
    height = Math.max(80, Math.min(window.innerHeight - 100, startHeight + delta));
  }

  function onMouseUp() {
    dragging = false;
  }
</script>

<svelte:window onmousemove={onMouseMove} onmouseup={onMouseUp} />

<div class="fixed bottom-0 left-0 right-0 z-50 border-t border-zinc-700 bg-zinc-950 flex flex-col" style="height: {height}px;">
  <!-- Drag handle -->
  <div
    class="h-1.5 w-full cursor-ns-resize shrink-0 bg-zinc-800 hover:bg-zinc-600 transition-colors"
    onmousedown={onDragStart}
  ></div>

  <div class="flex items-center justify-between px-3 py-1 border-b border-zinc-800 shrink-0">
    <span class="text-xs text-zinc-400 font-mono">Debug</span>
    <button
      onclick={() => debug.clear()}
      class="text-xs text-zinc-600 hover:text-zinc-300 cursor-pointer transition-colors"
    >Rensa</button>
  </div>
  <div class="flex-1 overflow-y-auto px-3 py-1 flex flex-col-reverse">
    {#each debug.logs as entry}
      <div class="flex gap-2 py-0.5 font-mono text-xs leading-relaxed">
        <span class="text-zinc-600 shrink-0">{entry.time}</span>
        <span class="text-zinc-300 whitespace-pre-wrap break-all">{entry.text}</span>
      </div>
    {:else}
      <p class="text-xs text-zinc-700 italic self-end">Inga loggar ännu</p>
    {/each}
  </div>
</div>
