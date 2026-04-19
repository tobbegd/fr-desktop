<script lang="ts">
  type Props = { items: string[]; onselect: (sql: string) => void; onremove: (sql: string) => void };
  let { items, onselect, onremove }: Props = $props();
</script>

<div class="flex flex-col gap-1 py-2">
  {#if items.length === 0}
    <p class="text-xs text-zinc-600 px-3">Ingen historik ännu.</p>
  {:else}
    {#each items as sql}
      <div class="group flex items-center justify-between px-3 py-1 rounded transition-colors hover:bg-zinc-800 cursor-pointer"
        onclick={() => onselect(sql)}
      >
        <span class="text-xs font-mono text-zinc-300 group-hover:text-white truncate flex-1">{sql.replace(/\n/g, " ")}</span>
        <button
          onclick={(e) => { e.stopPropagation(); onremove(sql); }}
          class="ml-2 text-zinc-600 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-all cursor-pointer text-xs shrink-0"
        >✕</button>
      </div>
    {/each}
  {/if}
</div>
