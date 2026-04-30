<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  type Sack = { id: number; namn: string; skapad: string; antal: number };
  type SackBolag = { orgnr: string; orgnamn: string; email: string };

  type Props = { navigateId?: number | null; onNavigated?: () => void };
  let { navigateId = null, onNavigated }: Props = $props();

  let sackar = $state<Sack[]>([]);
  let selected = $state<Sack | null>(null);
  let bolag = $state<SackBolag[]>([]);
  let nyttNamn = $state("");
  let skaparSack = $state(false);

  async function ladda() {
    sackar = await invoke<Sack[]>("list_sackar");
    if (selected) {
      selected = sackar.find(s => s.id === selected!.id) ?? null;
    }
  }

  async function laddaBolag(s: Sack) {
    selected = s;
    bolag = await invoke<SackBolag[]>("list_sack_bolag", { sackId: s.id });
  }

  async function skapaSack() {
    const namn = nyttNamn.trim();
    if (!namn) return;
    skaparSack = true;
    try {
      await invoke("create_sack", { namn });
      nyttNamn = "";
      await ladda();
    } finally {
      skaparSack = false;
    }
  }

  async function taBortSack(s: Sack) {
    await invoke("delete_sack", { id: s.id });
    if (selected?.id === s.id) { selected = null; bolag = []; }
    await ladda();
  }

  async function taBortBolag(orgnr: string) {
    if (!selected) return;
    await invoke("remove_bolag_from_sack", { sackId: selected.id, orgnr });
    bolag = bolag.filter(b => b.orgnr !== orgnr);
    await ladda();
  }

  onMount(async () => {
    await ladda();
    if (navigateId != null) {
      const s = sackar.find(s => s.id === navigateId);
      if (s) await laddaBolag(s);
      onNavigated?.();
    }
  });
</script>

<div class="flex h-full gap-0">
  <!-- Vänster: säcklista -->
  <div class="w-64 shrink-0 border-r border-zinc-800 flex flex-col">
    <div class="flex-1 overflow-y-auto">
      {#each sackar as s}
        <div
          role="button"
          tabindex="0"
          class="w-full text-left px-4 py-3 border-b border-zinc-800/60 flex items-center justify-between group transition-colors cursor-pointer
            {selected?.id === s.id ? 'bg-zinc-800' : 'bg-zinc-900/40 hover:bg-zinc-900'}"
          onclick={() => laddaBolag(s)}
          onkeydown={(e) => e.key === "Enter" && laddaBolag(s)}
        >
          <div>
            <p class="text-sm text-zinc-200">{s.namn}</p>
            <p class="text-xs text-zinc-500">{s.antal} bolag</p>
          </div>
          <button
            class="opacity-0 group-hover:opacity-100 text-zinc-600 hover:text-red-400 transition-all text-xs cursor-pointer px-1"
            onclick={(e) => { e.stopPropagation(); taBortSack(s); }}
          >✕</button>
        </div>
      {:else}
        <p class="text-xs text-zinc-600 px-4 py-3">Inga säckar ännu.</p>
      {/each}
    </div>

    <!-- Ny säck -->
    <div class="border-t border-zinc-800 p-3 flex gap-2">
      <input
        bind:value={nyttNamn}
        placeholder="Ny säck..."
        onkeydown={(e) => e.key === "Enter" && skapaSack()}
        class="flex-1 bg-zinc-900 border border-zinc-800 rounded px-2 py-1.5 text-xs text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600"
      />
      <button
        onclick={skapaSack}
        disabled={skaparSack || !nyttNamn.trim()}
        class="px-2.5 py-1.5 text-xs bg-zinc-700 hover:bg-zinc-600 text-zinc-200 rounded transition-colors cursor-pointer disabled:opacity-40"
      >+</button>
    </div>
  </div>

  <!-- Höger: bolag i vald säck -->
  <div class="flex-1 flex flex-col overflow-hidden">
    {#if selected}
      <div class="px-5 py-3 border-b border-zinc-800">
        <p class="text-sm font-medium text-zinc-200">{selected.namn}</p>
        <p class="text-xs text-zinc-500">{bolag.length} bolag</p>
      </div>
      <div class="flex-1 overflow-y-auto">
        {#each bolag as b}
          <div class="flex items-center justify-between px-5 py-2.5 border-b border-zinc-800/50 bg-zinc-900/40 group">
            <div>
              <p class="text-sm text-zinc-200">{b.orgnamn || b.orgnr}</p>
              <p class="text-xs text-zinc-500">{b.email || "–"}</p>
            </div>
            <button
              class="opacity-0 group-hover:opacity-100 text-zinc-600 hover:text-red-400 text-xs transition-all cursor-pointer px-1"
              onclick={() => taBortBolag(b.orgnr)}
            >✕</button>
          </div>
        {:else}
          <p class="text-xs text-zinc-600 px-5 py-4">Säcken är tom. Lägg till bolag från sökresultaten.</p>
        {/each}
      </div>
    {:else}
      <div class="flex-1 flex items-center justify-center">
        <p class="text-sm text-zinc-600">Välj en säck</p>
      </div>
    {/if}
  </div>
</div>
