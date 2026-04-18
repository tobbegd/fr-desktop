<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { AiExpl } from "$lib/aiPrompt";

  type Props = { dbPath: string };
  let { dbPath }: Props = $props();

  let schema = $state<Record<string, string[]>>({});
  let aiExpl = $state<AiExpl>({});
  let expanded = $state<Record<string, boolean>>({});

  $effect(() => {
    if (dbPath) {
      invoke<Record<string, string[]>>("get_schema", { dbPath })
        .then(s => { schema = s; })
        .catch(() => {});
      invoke<AiExpl>("get_ai_explanations", { dbPath })
        .then(e => { aiExpl = e; })
        .catch(() => {});
    }
  });

  function toggle(table: string) {
    expanded[table] = !expanded[table];
  }
</script>

<div class="flex flex-col gap-1 py-2">
  {#if Object.keys(schema).length === 0}
    <p class="text-xs text-zinc-600 px-3">Ingen databas laddad.</p>
  {:else}
    {#each Object.entries(schema) as [table, cols]}
      <div>
        <button
          class="w-full flex items-center justify-between px-3 py-1 text-xs font-mono text-zinc-300 hover:text-white hover:bg-zinc-800 rounded transition-colors cursor-pointer select-none"
          onclick={() => toggle(table)}
        >
          <span>{table}</span>
          <span class="text-zinc-600">{expanded[table] ? "▲" : "▼"} {cols.length} kol</span>
        </button>
        {#if expanded[table]}
          <div class="flex flex-col gap-0.5 px-5 pb-1">
            {#each cols as col}
              {@const desc = aiExpl[table]?.[col]}
              <div class="flex items-baseline gap-2 py-0.5">
                <span class="text-xs font-mono text-zinc-400 shrink-0">{col}</span>
                {#if desc}
                  <span class="text-xs text-zinc-600 truncate">{desc}</span>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  {/if}
</div>
