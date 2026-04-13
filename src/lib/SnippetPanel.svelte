<script lang="ts">
  import { Store } from "@tauri-apps/plugin-store";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  type Props = { currentSql: string; onselect: (sql: string) => void };
  let { currentSql, onselect }: Props = $props();

  type Snippet = { id: string; name: string; sql: string };
  type Tab = { id: string; name: string; snippets: Snippet[] };

  const STANDARD_ID = "standard";

  const predefined: Snippet[] = [
    { id: "pre_1", name: "Alla bolag (100)", sql: "SELECT *\nFROM bolag\nLIMIT 100" },
    { id: "pre_2", name: "Räkna bolag", sql: "SELECT COUNT(*) AS antal\nFROM bolag" },
    { id: "pre_3", name: "Sök på namn", sql: "SELECT *\nFROM bolag\nWHERE orgnamn LIKE '%sök%'\nLIMIT 100" },
    { id: "pre_4", name: "Bolag med kontaktuppgifter", sql: "SELECT orgnr, orgnamn, telefon, email, webbadress, postort\nFROM bolag\nWHERE telefon IS NOT NULL\n  AND orgnamn LIKE '%sök%'\nLIMIT 100" },
    { id: "pre_5", name: "Bolag med SNI-bransch", sql: "SELECT orgnr, orgnamn, sni_1, sni_1_namn, postort\nFROM bolag\nWHERE sni_1 != ''\nLIMIT 100" },
    { id: "pre_8", name: "Bolag per storleksklass (anst)", sql: "SELECT storleksklass_anst, COUNT(*) AS antal\nFROM bolag\nWHERE storleksklass_anst IS NOT NULL\nGROUP BY storleksklass_anst\nORDER BY storleksklass_anst" },
    { id: "pre_9", name: "Registrerade arbetsgivare med kontakt", sql: "SELECT orgnr, orgnamn, telefon, email, storleksklass_anst,\n       antal_arbetsst, postort\nFROM bolag\nWHERE arbetsgivarstatus IN ('1','2','3')\n  AND telefon IS NOT NULL\nLIMIT 100" },
    { id: "pre_10", name: "Exporterande bolag", sql: "SELECT orgnr, orgnamn, export_import, telefon, email, postort\nFROM bolag\nWHERE export_import = '2'\nLIMIT 100" },
    { id: "pre_6", name: "Årsredovisningar senaste år", sql: "SELECT b.orgnamn, a.rakenskapsar_slut, a.nettoomsattning,\n       a.arets_resultat, a.medelantal_anstallda\nFROM arsredovisningar a\nJOIN bolag b ON b.orgnr = a.orgnr\nORDER BY a.rakenskapsar_slut DESC\nLIMIT 100" },
    { id: "pre_7", name: "Visa tabeller", sql: "SELECT name\nFROM sqlite_master\nWHERE type='table'\nORDER BY name" },
  ];

  let tabs = $state<Tab[]>([]);
  let activeTabId = $state<string>(STANDARD_ID);
  let deletedPredefined = $state(new Set<string>());
  let search = $state("");
  let addingName = $state("");
  let showAddInput = $state(false);
  let renamingTabId = $state<string | null>(null);
  let renamingName = $state("");
  let store: Store | null = null;
  let fileInput: HTMLInputElement;
  let importError = $state("");
  let confirmDeleteTabId = $state<string | null>(null);
  const confirmDeleteTab = $derived(tabs.find(t => t.id === confirmDeleteTabId) ?? null);

  const isStandard = $derived(activeTabId === STANDARD_ID);
  const activeTab = $derived(isStandard ? null : tabs.find(t => t.id === activeTabId) ?? null);

  const visibleSnippets = $derived(
    isStandard
      ? predefined.filter(p => !deletedPredefined.has(p.id))
      : (activeTab?.snippets ?? [])
  );

  const filtered = $derived(visibleSnippets.filter(s => fuzzyMatch(s.name, search)));

  onMount(async () => {
    store = await Store.load("snippets.json");

    const savedTabs = await store.get<Tab[]>("tabs");
    const savedActiveTabId = await store.get<string>("activeTabId");
    const deleted = await store.get<string[]>("deletedPredefined");

    if (deleted) deletedPredefined = new Set(deleted);

    if (savedTabs && savedTabs.length > 0) {
      tabs = savedTabs;
      const validId = savedActiveTabId === STANDARD_ID || savedTabs.some(t => t.id === savedActiveTabId);
      activeTabId = validId && savedActiveTabId ? savedActiveTabId : STANDARD_ID;
    } else {
      // Migrera från gammalt platt format
      const oldSnippets = await store.get<Snippet[]>("snippets");
      const defaultTab: Tab = { id: crypto.randomUUID(), name: "Mina snippets", snippets: oldSnippets ?? [] };
      tabs = [defaultTab];
      activeTabId = STANDARD_ID;
      await persist();
    }
  });

  async function persist() {
    await store?.set("tabs", tabs);
    await store?.set("activeTabId", activeTabId);
    await store?.set("deletedPredefined", [...deletedPredefined]);
    await store?.save();
  }

  function fuzzyMatch(name: string, query: string): boolean {
    if (!query) return true;
    const n = name.toLowerCase();
    return query.toLowerCase().split(" ").every(w => n.includes(w));
  }

  async function addSnippet() {
    const name = addingName.trim();
    if (!name || !currentSql.trim() || !activeTab) return;
    const snippet: Snippet = { id: crypto.randomUUID(), name, sql: currentSql.trim() };
    tabs = tabs.map(t => t.id === activeTab.id ? { ...t, snippets: [...t.snippets, snippet] } : t);
    await persist();
    addingName = "";
    showAddInput = false;
  }

  async function deleteSnippet(id: string) {
    if (isStandard) {
      deletedPredefined = new Set([...deletedPredefined, id]);
    } else if (activeTab) {
      tabs = tabs.map(t => t.id === activeTab.id
        ? { ...t, snippets: t.snippets.filter(s => s.id !== id) }
        : t
      );
    }
    await persist();
  }

  async function addTab() {
    const tab: Tab = { id: crypto.randomUUID(), name: "Ny grupp", snippets: [] };
    tabs = [...tabs, tab];
    activeTabId = tab.id;
    await persist();
    renamingTabId = tab.id;
    renamingName = "Ny grupp";
  }

  async function deleteTab(id: string) {
    tabs = tabs.filter(t => t.id !== id);
    if (activeTabId === id) activeTabId = tabs.length > 0 ? tabs[0].id : STANDARD_ID;
    await persist();
  }

  function startRename(tab: Tab) {
    renamingTabId = tab.id;
    renamingName = tab.name;
  }

  async function commitRename() {
    const name = renamingName.trim();
    if (name && renamingTabId) {
      tabs = tabs.map(t => t.id === renamingTabId ? { ...t, name } : t);
      await persist();
    }
    renamingTabId = null;
  }

  async function saveTabToFile() {
    const data = isStandard
      ? { version: 1, name: "Standard", snippets: predefined.filter(p => !deletedPredefined.has(p.id)) }
      : { version: 1, name: activeTab!.name, snippets: activeTab!.snippets };
    const filename = `${data.name.toLowerCase().replace(/\s+/g, "_")}_snippets.json`;
    await invoke("save_file", { filename, content: JSON.stringify(data, null, 2) }).catch(() => {});
  }

  function triggerImport() {
    importError = "";
    fileInput.click();
  }

  async function onFileSelected(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    try {
      const data = JSON.parse(await file.text());
      if (!data.name || !Array.isArray(data.snippets)) throw new Error();
      const tab: Tab = {
        id: crypto.randomUUID(),
        name: data.name,
        snippets: data.snippets.map((s: any) => ({
          id: crypto.randomUUID(),
          name: String(s.name ?? ""),
          sql: String(s.sql ?? ""),
        })).filter((s: Snippet) => s.name && s.sql),
      };
      tabs = [...tabs, tab];
      activeTabId = tab.id;
      await persist();
    } catch {
      importError = "Filen kunde inte läsas in.";
      setTimeout(() => importError = "", 3000);
    }
    (e.target as HTMLInputElement).value = "";
  }

  function onSaveKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") addSnippet();
    if (e.key === "Escape") { showAddInput = false; addingName = ""; }
  }

  function onRenameKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") commitRename();
    if (e.key === "Escape") { renamingTabId = null; }
  }

  function selectAll(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

<div class="border border-zinc-700 border-t-0 rounded-b-lg bg-zinc-900 overflow-hidden">

  <!-- Tabbar -->
  <div class="flex items-center gap-0.5 px-2 pt-1.5 pb-0">
    <!-- Standard-tab -->
    <button
      class="shrink-0 px-2.5 py-1 text-xs rounded-t transition-colors cursor-pointer whitespace-nowrap
        {activeTabId === STANDARD_ID
          ? 'bg-zinc-900 text-zinc-200 border border-zinc-700 border-b-zinc-900 -mb-px relative z-10'
          : 'text-zinc-500 hover:text-zinc-300'}"
      onclick={() => { activeTabId = STANDARD_ID; persist(); }}
    >Standard</button>

    <!-- Användartabs -->
    {#each tabs as tab}
      <div
        class="group shrink-0 flex items-center gap-1 px-2 py-1 rounded-t text-xs cursor-pointer whitespace-nowrap transition-colors
          {activeTabId === tab.id
            ? 'bg-zinc-900 text-zinc-200 border border-zinc-700 border-b-zinc-900 -mb-px relative z-10'
            : 'text-zinc-500 hover:text-zinc-300'}"
        onclick={() => { activeTabId = tab.id; persist(); }}
      >
        {#if renamingTabId === tab.id}
          <input
            type="text"
            bind:value={renamingName}
            onkeydown={onRenameKeydown}
            onblur={commitRename}
            use:selectAll
            class="w-24 bg-transparent text-xs text-white outline-none border-b border-amber-500"
            onclick={(e) => e.stopPropagation()}
          />
        {:else}
          <span ondblclick={(e) => { e.stopPropagation(); startRename(tab); }}>{tab.name}</span>
        {/if}

        {#if activeTabId === tab.id && renamingTabId !== tab.id}
          <button
            title="Spara tab till fil"
            onclick={(e) => { e.stopPropagation(); saveTabToFile(); }}
            class="text-base px-0.5 text-zinc-500 hover:text-zinc-200 cursor-pointer transition-colors leading-none"
          >↓</button>
          <button
            title="Ta bort tab"
            onclick={(e) => { e.stopPropagation(); confirmDeleteTabId = tab.id; }}
            class="text-base px-0.5 text-zinc-600 hover:text-red-400 cursor-pointer transition-colors leading-none"
          >×</button>
        {/if}
      </div>
    {/each}

    <!-- Ny tab + importera -->
    <div class="flex items-center gap-1 ml-1 shrink-0">
      <button
        title="Ny grupp"
        onclick={addTab}
        class="px-1 py-1 text-base text-zinc-500 hover:text-zinc-200 cursor-pointer transition-colors leading-none"
      >+</button>
      <button
        title="Ladda tab från fil"
        onclick={triggerImport}
        class="px-1 py-1 text-base text-zinc-500 hover:text-zinc-200 cursor-pointer transition-colors leading-none"
      >↑</button>
    </div>

    {#if isStandard}
      <button
        title="Spara Standard-tab till fil"
        onclick={saveTabToFile}
        class="ml-auto shrink-0 px-1.5 py-1 text-xs text-zinc-600 hover:text-zinc-300 cursor-pointer transition-colors"
      >↓</button>
    {/if}
  </div>

  <!-- Sökrad + spara-knapp -->
  <div class="flex items-center gap-2 px-3 py-2 border-t border-zinc-700 border-b border-zinc-800">
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
        onkeydown={onSaveKeydown}
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
        disabled={!currentSql.trim() || isStandard}
        title={isStandard ? "Välj en grupp för att spara snippets" : "Spara nuvarande SQL som snippet"}
        class="px-2 py-1 text-xs text-zinc-400 hover:text-white disabled:opacity-30 cursor-pointer disabled:cursor-default transition-colors whitespace-nowrap"
      >+ Spara nuvarande</button>
    {/if}
  </div>

  <!-- Snippet-lista -->
  <div class="max-h-48 overflow-y-auto">
    {#if importError}
      <p class="px-3 py-2 text-xs text-red-400">{importError}</p>
    {/if}
    {#if filtered.length === 0}
      <p class="px-3 py-3 text-xs text-zinc-600">
        {isStandard ? "Inga snippets." : "Inga snippets i denna grupp."}
      </p>
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

{#if confirmDeleteTab}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
    <div class="bg-zinc-900 border border-zinc-700 rounded-lg p-5 w-72 flex flex-col gap-4 shadow-xl">
      <p class="text-sm text-zinc-200 font-medium">Ta bort "{confirmDeleteTab.name}"?</p>
      <p class="text-xs text-zinc-400">
        Alla {confirmDeleteTab.snippets.length} sparade {confirmDeleteTab.snippets.length === 1 ? "snippet" : "snippets"} i gruppen tas bort permanent.
        Spara tabben till fil först om du vill behålla dem.
      </p>
      <div class="flex gap-2 justify-end">
        <button
          onclick={() => confirmDeleteTabId = null}
          class="px-3 py-1.5 text-xs rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300 cursor-pointer transition-colors"
        >Nej, avbryt</button>
        <button
          onclick={() => { deleteTab(confirmDeleteTab!.id); confirmDeleteTabId = null; }}
          class="px-3 py-1.5 text-xs rounded bg-red-700 hover:bg-red-600 text-white cursor-pointer transition-colors"
        >Ja, ta bort</button>
      </div>
    </div>
  </div>
{/if}

<input bind:this={fileInput} type="file" accept=".json" onchange={onFileSelected} class="hidden" />
