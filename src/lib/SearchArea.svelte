<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import SqlEditor from "./SqlEditor.svelte";

  type Props = { dbPath: string };
  let { dbPath }: Props = $props();

  type SearchMode = { id: string; label: string };
  const modes: SearchMode[] = [
    { id: "sql", label: "SQL" },
    // { id: "ai", label: "AI" },
  ];

  let activeMode = $state<string | null>(null);
  let sqlQuery = $state("");
  let schema = $state<Record<string, string[]>>({});

  async function loadSchema() {
    if (!dbPath) return;
    try {
      schema = await invoke<Record<string, string[]>>("get_schema", { dbPath });
    } catch {}
  }

  $effect(() => {
    if (dbPath) loadSchema();
  });
  let running = $state(false);
  let error = $state("");
  let result = $state<{ columns: string[]; rows: unknown[][]; truncated: boolean } | null>(null);
  let pageSize = $state(25);
  let currentPage = $state(0);

  const pageSizes = [25, 50, 100, 200];
  const totalPages = $derived(result ? Math.ceil(result.rows.length / pageSize) : 0);
  const pagedRows = $derived(result ? result.rows.slice(currentPage * pageSize, (currentPage + 1) * pageSize) : []);

  let sortKeys = $state<{ col: string; dir: "ASC" | "DESC" }[]>([]);

  function stripOrderBy(sql: string): string {
    return sql.replace(/\s+ORDER\s+BY\s+[\s\S]+$/i, "").trim();
  }

  function handleHeaderClick(col: string) {
    const idx = sortKeys.findIndex(k => k.col === col);
    if (idx === -1) {
      sortKeys = [...sortKeys, { col, dir: "ASC" }];
    } else if (sortKeys[idx].dir === "ASC") {
      sortKeys = sortKeys.map((k, i) => i === idx ? { ...k, dir: "DESC" } : k);
    } else {
      sortKeys = sortKeys.filter((_, i) => i !== idx);
    }
    if (sortKeys.length === 0) {
      sqlQuery = stripOrderBy(sqlQuery);
    } else {
      const orderBy = sortKeys.map(k => `"${k.col}" ${k.dir}`).join(", ");
      sqlQuery = `${stripOrderBy(sqlQuery)}\nORDER BY ${orderBy}`;
    }
    runQuery();
  }

  function toggleMode(id: string) {
    activeMode = activeMode === id ? null : id;
  }

  async function runQuery() {
    if (!sqlQuery.trim() || !dbPath) return;
    running = true;
    error = "";
    result = null;
    currentPage = 0;
    sortKeys = [];
    try {
      result = await invoke<{ columns: string[]; rows: unknown[][]; truncated: boolean }>(
        "query_db",
        { dbPath, sql: sqlQuery }
      );
    } catch (e) {
      error = String(e);
    } finally {
      running = false;
    }
  }

</script>

<!-- SearchArea fyller återstående höjd i föräldern -->
<div class="flex-1 flex flex-col min-h-0">

  <!-- Sök-input (fast höjd) -->
  <div class="shrink-0 border-b border-zinc-800">
    <div class="flex items-center gap-1 px-3 py-1.5">
      <span class="text-xs font-bold text-zinc-600 mr-1">Sök</span>
      {#each modes as mode}
        <button
          class="px-2.5 py-1 text-xs rounded-md transition-colors cursor-pointer
            {activeMode === mode.id
              ? 'bg-zinc-700 text-white'
              : 'text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50'}"
          onclick={() => toggleMode(mode.id)}
        >
          {mode.label}
        </button>
      {/each}
    </div>

    {#if activeMode === "sql"}
      <div class="px-3 pb-3 flex flex-col gap-2">
        <SqlEditor
          bind:value={sqlQuery}
          {schema}
          onchange={(v) => (sqlQuery = v)}
          onrun={runQuery}
        />
        <div class="flex items-center justify-between">
          <span class="text-xs text-zinc-600">Ctrl+Enter för att köra</span>
          <button
            class="px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded-md hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-40"
            disabled={!sqlQuery.trim() || running || !dbPath}
            onclick={runQuery}
          >
            {running ? "Kör..." : "Kör"}
          </button>
        </div>
        {#if !dbPath}
          <p class="text-xs text-zinc-600">Ingen databas nedladdad ännu.</p>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Resultat -->
  {#if error}
    <div class="px-4 py-3 text-sm text-red-400 font-mono">{error}</div>
  {:else if result}
    <div class="flex-1 flex flex-col min-h-0">

      {#if result.rows.length === 0}
        <p class="px-4 py-3 text-sm text-zinc-500">Inga rader.</p>
      {:else}
        <!-- Varning (fast, scrollar inte) -->
        {#if result.truncated}
          <p class="shrink-0 px-4 py-1.5 text-xs text-yellow-600 border-b border-zinc-800">
            Visar max 200 rader — lägg till LIMIT i din query för fler.
          </p>
        {/if}

        <!-- Tabell (scrollar, sticky header fungerar här) -->
        <div class="flex-1 overflow-auto min-h-0">
          <table class="w-full text-xs text-left border-collapse">
            <thead>
              <tr>
                {#each result.columns as col}
                  <th
                    class="sticky top-0 bg-zinc-950 px-3 py-2 font-medium whitespace-nowrap border-b border-zinc-800 z-10 cursor-pointer select-none
                      {sortKeys.some(k => k.col === col) ? 'text-white' : 'text-zinc-400 hover:text-zinc-200'}"
                    onclick={() => handleHeaderClick(col)}
                  >
                    {col}
                    {#if sortKeys.findIndex(k => k.col === col) !== -1}
                      {@const k = sortKeys.find(k => k.col === col)!}
                      {@const pos = sortKeys.findIndex(k => k.col === col) + 1}
                      <span class="ml-1 text-zinc-400">
                        {k.dir === "ASC" ? "↑" : "↓"}{sortKeys.length > 1 ? pos : ""}
                      </span>
                    {/if}
                  </th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each pagedRows as row, i}
                <tr class="border-b border-zinc-900 {i % 2 === 0 ? '' : 'bg-zinc-900/30'}">
                  {#each row as cell}
                    <td class="px-3 py-1.5 text-zinc-300 whitespace-nowrap max-w-xs truncate">
                      {#if cell === null}
                        <span class="text-zinc-600">NULL</span>
                      {:else}
                        {String(cell)}
                      {/if}
                    </td>
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        <!-- Sidfot (fast, scrollar inte) -->
        <div class="shrink-0 flex items-center justify-between px-4 py-2 border-t border-zinc-800 text-xs text-zinc-500">
          <span>
            {currentPage * pageSize + 1}–{Math.min((currentPage + 1) * pageSize, result.rows.length)}
            av {result.rows.length} rader
          </span>
          <div class="flex items-center gap-3">
            <label class="flex items-center gap-1.5">
              Rader per sida
              <select
                bind:value={pageSize}
                onchange={() => currentPage = 0}
                class="bg-zinc-800 border border-zinc-700 rounded px-1.5 py-0.5 text-zinc-300 cursor-pointer"
              >
                {#each pageSizes as s}
                  <option value={s}>{s}</option>
                {/each}
              </select>
            </label>
            <div class="flex items-center gap-1">
              <button
                class="px-2 py-0.5 rounded hover:bg-zinc-800 disabled:opacity-30 cursor-pointer disabled:cursor-default transition-colors"
                disabled={currentPage === 0}
                onclick={() => currentPage--}
              >←</button>
              <span>{currentPage + 1} / {totalPages}</span>
              <button
                class="px-2 py-0.5 rounded hover:bg-zinc-800 disabled:opacity-30 cursor-pointer disabled:cursor-default transition-colors"
                disabled={currentPage >= totalPages - 1}
                onclick={() => currentPage++}
              >→</button>
            </div>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
