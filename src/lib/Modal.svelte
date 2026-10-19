<script lang="ts">
  import type { Snippet } from "svelte";

  // Delad modal-wrapper (overlay + fast header med titel/undertitel/stäng +
  // ESC-stöd) — samma mönster som tidigare dupplicerades i NyckeltaPanel och
  // KartaPanel. Nya paneler (t.ex. KontakterPanel) ska använda denna istället
  // för att bygga egen overlay/header, så framtida justeringar (storlek,
  // animation, tangentbordsstöd) bara behöver göras på ett ställe.
  type Props = {
    title: string;
    subtitle?: string;
    onclose: () => void;
    headerExtra?: Snippet;
    children: Snippet;
  };
  let { title, subtitle, onclose, headerExtra, children }: Props = $props();

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<!-- Overlay -->
<div
  class="fixed inset-0 z-40 bg-black/60"
  role="button"
  tabindex="-1"
  onclick={onclose}
  onkeydown={() => {}}
></div>

<!-- Panel -->
<div class="fixed inset-4 z-50 flex flex-col rounded-xl border border-zinc-700 bg-zinc-950 shadow-2xl overflow-hidden">
  <!-- Header -->
  <div class="flex items-center gap-4 px-5 py-3 border-b border-zinc-800 shrink-0">
    <div>
      <span class="text-base font-semibold text-zinc-100">{title}</span>
      {#if subtitle}
        <span class="ml-3 text-sm text-zinc-500">{subtitle}</span>
      {/if}
    </div>
    <div class="flex-1"></div>
    {#if headerExtra}
      {@render headerExtra()}
    {/if}
    <button
      class="text-zinc-400 hover:text-zinc-100 text-xl leading-none px-1 cursor-pointer"
      onclick={onclose}
    >✕</button>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-y-auto p-5">
    {@render children()}
  </div>
</div>
