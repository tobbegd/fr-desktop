<script lang="ts">
  import { Store } from "@tauri-apps/plugin-store";
  import { onMount } from "svelte";

  type Props = { currentSql: string; onselect: (sql: string) => void };
  let { currentSql, onselect }: Props = $props();

  type Snippet = { id: string; name: string; sql: string };

  const predefined: Snippet[] = [
    { id: "pre_1", name: "Alla bolag (100)", sql: "SELECT *\nFROM bolag\nLIMIT 100" },
    { id: "pre_2", name: "Räkna bolag", sql: "SELECT COUNT(*) AS antal\nFROM bolag" },
    { id: "pre_3", name: "Sök på namn", sql: "SELECT *\nFROM bolag\nWHERE orgnamn LIKE '%sök%'\nLIMIT 100" },
    { id: "pre_4", name: "Bolag med webbplats", sql: "SELECT b.orgnr, b.orgnamn, w.domain, w.email, w.phone\nFROM bolag b\nJOIN webbplatser w ON w.orgnr = b.orgnr\nWHERE b.orgnamn LIKE '%Bil%'\n  AND w.phone IS NOT NULL AND w.phone != ''\nLIMIT 100" },
    { id: "pre_5", name: "Bolag med SNI-bransch", sql: "SELECT orgnr, orgnamn, sni_1, sni_1_namn, postort\nFROM bolag\nWHERE sni_1 != ''\nLIMIT 100" },
    { id: "pre_6", name: "Årsredovisningar senaste år", sql: "SELECT b.orgnamn, a.rakenskapsar_slut, a.nettoomsattning,\n       a.arets_resultat, a.medelantal_anstallda\nFROM arsredovisningar a\nJOIN bolag b ON b.orgnr = a.orgnr\nORDER BY a.rakenskapsar_slut DESC\nLIMIT 100" },
    { id: "pre_7", name: "Visa tabeller", sql: "SELECT name\nFROM sqlite_master\nWHERE type='table'\nORDER BY name" },
  ];

  let userSnippets = $state<Snippet[]>([]);
  let search = $state("");
  let addingName = $state("");
  let showAddInput = $state(false);
  let deletedPredefined = $state(new Set<string>());
  let store: Store | null = null;

  onMount(async () => {
    store = await Store.load("snippets.json");
    const saved = await store.get<Snippet[]>("snippets");
    if (saved) userSnippets = saved;
    const deleted = await store.get<string[]>("deletedPredefined");
    if (deleted) deletedPredefined = new Set(deleted);
  });

  async function saveToStore() {
    await store?.set("snippets", userSnippets);
    await store?.set("deletedPredefined", [...deletedPredefined]);
    await store?.save();
  }

  function fuzzyMatch(name: string, query: string): boolean {
    if (!query) return true;
    const n = name.toLowerCase();
    return query.toLowerCase().split(" ").every(word => n.includes(word));
  }

  const allSnippets = $derived([...predefined.filter(p => !deletedPredefined.has(p.id)), ...userSnippets]);
  const filtered = $derived(allSnippets.filter(s => fuzzyMatch(s.name, search)));

  async function addSnippet() {
    const name = addingName.trim();
    if (!name || !currentSql.trim()) return;
    const snippet: Snippet = { id: crypto.randomUUID(), name, sql: currentSql.trim() };
    userSnippets = [...userSnippets, snippet];
    await saveToStore();
    addingName = "";
    showAddInput = false;
  }

  async function deleteSnippet(id: string) {
    if (predefined.some(p => p.id === id)) {
      deletedPredefined = new Set([...deletedPredefined, id]);
    } else {
      userSnippets = userSnippets.filter(s => s.id !== id);
    }
    await saveToStore();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Enter") addSnippet();
    if (e.key === "Escape") { showAddInput = false; addingName = ""; }
  }
</script>

<div class="border border-zinc-700 border-t-0 rounded-b-lg bg-zinc-900 overflow-hidden">
  <!-- Sökrad + spara-knapp -->
  <div class="flex items-center gap-2 px-3 py-2 border-b border-zinc-800">
    <input
      type="text"
      bind:value={search}
      placeholder="Sök snippet..."
      class="flex-1 bg-zinc-800 border border-zinc-700 rounded px-2 py-1 text-xs text-white placeholder-zinc-600 focus:outline-none focus:border-zinc-500"
    />
    {#if showAddInput}
      <input
        type="text"
        bind:value={addingName}
        onkeydown={onkeydown}
        placeholder="Namn på snippet..."
        autofocus
        class="w-40 bg-zinc-800 border border-amber-600 rounded px-2 py-1 text-xs text-white placeholder-zinc-600 focus:outline-none"
      />
      <button
        onclick={addSnippet}
        disabled={!addingName.trim() || !currentSql.trim()}
        class="px-2 py-1 text-xs bg-amber-600 text-white rounded hover:bg-amber-500 disabled:opacity-40 cursor-pointer disabled:cursor-default transition-colors"
      >Spara</button>
      <button
        onclick={() => { showAddInput = false; addingName = ""; }}
        class="px-2 py-1 text-xs text-zinc-400 hover:text-white cursor-pointer transition-colors"
      >✕</button>
    {:else}
      <button
        onclick={() => showAddInput = true}
        disabled={!currentSql.trim()}
        title="Spara nuvarande SQL som snippet"
        class="px-2 py-1 text-xs text-zinc-400 hover:text-white disabled:opacity-40 cursor-pointer disabled:cursor-default transition-colors whitespace-nowrap"
      >+ Spara nuvarande</button>
    {/if}
  </div>

  <!-- Snippet-lista -->
  <div class="max-h-48 overflow-y-auto">
    {#if filtered.length === 0}
      <p class="px-3 py-3 text-xs text-zinc-600">Inga snippets matchar.</p>
    {:else}
      {#each filtered as snippet}
        <div
          class="group flex items-center justify-between px-3 py-2 hover:bg-zinc-800 cursor-pointer border-b border-zinc-800/50 last:border-0 transition-colors"
          onclick={() => onselect(snippet.sql)}
        >
          <div class="flex-1 min-w-0">
            <span class="text-xs text-zinc-200">{snippet.name}</span>
            <span class="ml-2 text-xs text-zinc-600 truncate">{snippet.sql.replace(/\n/g, " ")}</span>
          </div>
          <button
            onclick={(e) => { e.stopPropagation(); deleteSnippet(snippet.id); }}
            class="ml-2 mr-3 text-zinc-600 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-all cursor-pointer text-xs"
          >✕</button>
        </div>
      {/each}
    {/if}
  </div>
</div>
