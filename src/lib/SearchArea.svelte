<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import SqlEditor from "./SqlEditor.svelte";
  import ContextMenu from "./ContextMenu.svelte";
  import SnippetPanel from "./SnippetPanel.svelte";

  type Props = { dbPath: string };
  let { dbPath }: Props = $props();

  type SearchMode = { id: string; label: string };
  const modes: SearchMode[] = [
    { id: "sql", label: "SQL" },
    // { id: "ai", label: "AI" },
  ];

  let activeMode = $state<string | null>(null);
  let showSnippets = $state(false);
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
  let pageSize = $state(200);
  let currentPage = $state(0);

  const pageSizes = [200, 400, 800];

  let selectedRows = $state(new Set<number>());
  let excludedRows = $state(new Set<number>());
  let contextMenu = $state<{ x: number; y: number; rowIdx: number } | null>(null);

  const filteredRows = $derived(
    result ? result.rows.map((row, i) => ({ row, i })).filter(({ i }) => !excludedRows.has(i)) : []
  );
  const totalPages = $derived(Math.ceil(filteredRows.length / pageSize));
  const pagedRows = $derived(filteredRows.slice(currentPage * pageSize, (currentPage + 1) * pageSize));

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

  let lastClickedRow = $state<number | null>(null);

  function toggleRowSelect(i: number, shift: boolean) {
    const s = new Set(selectedRows);
    if (shift && lastClickedRow !== null) {
      const from = Math.min(lastClickedRow, i);
      const to = Math.max(lastClickedRow, i);
      const rangeIndices = filteredRows.slice(from, to + 1).map(r => r.i);
      rangeIndices.forEach(idx => s.add(idx));
    } else {
      s.has(i) ? s.delete(i) : s.add(i);
      lastClickedRow = i;
    }
    selectedRows = s;
  }

  function openContextMenu(e: MouseEvent, rowIdx: number) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, rowIdx };
  }

  function deleteRow(i: number) {
    excludedRows = new Set([...excludedRows, i]);
    selectedRows = new Set([...selectedRows].filter(r => r !== i));
    currentPage = Math.min(currentPage, Math.max(0, totalPages - 1));
  }

  function deleteSelected() {
    excludedRows = new Set([...excludedRows, ...selectedRows]);
    selectedRows = new Set();
    currentPage = 0;
  }

  function deleteUnselected() {
    const toExclude = filteredRows.filter(({ i }) => !selectedRows.has(i)).map(({ i }) => i);
    excludedRows = new Set([...excludedRows, ...toExclude]);
    currentPage = 0;
  }

  function restoreAll() {
    excludedRows = new Set();
    currentPage = 0;
  }

  function toCSV(rows: unknown[][], withHeader: boolean): string {
    function escapeCell(val: unknown): string {
      if (val === null || val === undefined) return "";
      const s = String(val);
      return s.includes(",") || s.includes('"') || s.includes("\n")
        ? `"${s.replace(/"/g, '""')}"`
        : s;
    }
    const lines: string[] = [];
    if (withHeader && result) lines.push(result.columns.map(escapeCell).join(","));
    for (const row of rows) lines.push(row.map(escapeCell).join(","));
    return lines.join("\n");
  }

  function copySelectedAsCSV(withHeader: boolean) {
    if (!result) return;
    const rows = filteredRows.filter(({ i }) => selectedRows.has(i)).map(({ row }) => row);
    navigator.clipboard.writeText(toCSV(rows, withHeader));
  }

  function toggleMode(id: string) {
    activeMode = activeMode === id ? null : id;
  }

  async function runQuery(resetSort = false) {
    if (!sqlQuery.trim() || !dbPath) return;
    running = true;
    error = "";
    result = null;
    currentPage = 0;
    selectedRows = new Set();
    excludedRows = new Set();
    lastClickedRow = null;
    if (resetSort) sortKeys = [];
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
        <div class="relative">
          <SqlEditor
            bind:value={sqlQuery}
            {schema}
            onchange={(v) => (sqlQuery = v)}
            onrun={() => runQuery(true)}
          />
          <button
            onclick={() => showSnippets = !showSnippets}
            title={showSnippets ? "Dölj snippets" : "Visa snippets"}
            class="absolute -bottom-3 left-1/2 -translate-x-1/2 z-10 px-3 py-0.5 text-xs rounded-full border border-zinc-700 bg-zinc-900 text-zinc-500 hover:text-zinc-300 hover:border-zinc-500 transition-colors cursor-pointer select-none"
          >{showSnippets ? "▲" : "▼"}</button>
        </div>
        {#if showSnippets}
          <div class="mt-1">
            <SnippetPanel
              currentSql={sqlQuery}
              onselect={(sql) => { sqlQuery = sql; showSnippets = false; runQuery(true); }}
            />
          </div>
        {/if}
        <div class="flex items-center justify-between">
          <span class="text-xs text-zinc-600">Ctrl+Enter för att köra</span>
          <button
            class="px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded-md hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-40"
            disabled={!sqlQuery.trim() || running || !dbPath}
            onclick={() => runQuery(true)}
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
            Resultat avkortat till 50 000 rader — förfina din query med WHERE eller LIMIT.
          </p>
        {/if}

        <!-- Tabell (scrollar, sticky header fungerar här) -->
        <div class="flex-1 overflow-auto min-h-0">
          <table class="w-full text-xs text-left border-collapse">
            <thead>
              <tr>
                {#each result.columns as col}
                  <th
                    class="sticky top-0 px-3 py-2 font-medium whitespace-nowrap border-b border-zinc-800 z-10 cursor-pointer select-none transition-colors
                      {sortKeys.some(k => k.col === col)
                        ? 'bg-zinc-800 text-white'
                        : 'bg-zinc-950 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'}"
                    onclick={() => handleHeaderClick(col)}
                  >
                    {col}
                    {#if sortKeys.findIndex(k => k.col === col) !== -1}
                      {@const k = sortKeys.find(k => k.col === col)!}
                      {@const pos = sortKeys.findIndex(k => k.col === col) + 1}
                      <span class="ml-1 text-amber-400">
                        {k.dir === "ASC" ? "↑" : "↓"}{sortKeys.length > 1 ? pos : ""}
                      </span>
                    {/if}
                  </th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each pagedRows as { row, i }}
                <tr
                  class="border-b border-zinc-900 cursor-pointer select-none transition-colors
                    {selectedRows.has(i) ? 'bg-amber-900/40' : i % 2 === 0 ? 'hover:bg-zinc-800/40' : 'bg-zinc-900/30 hover:bg-zinc-800/40'}"
                  onclick={(e) => toggleRowSelect(i, e.shiftKey)}
                  oncontextmenu={(e) => openContextMenu(e, i)}
                >
                  {#each row as cell}
                    <td class="px-3 py-1.5 whitespace-nowrap max-w-xs truncate
                      {selectedRows.has(i) ? 'text-white' : 'text-zinc-300'}">
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
            {currentPage * pageSize + 1}–{Math.min((currentPage + 1) * pageSize, filteredRows.length)}
            av {filteredRows.length} rader
            {#if excludedRows.size > 0}
              <span class="text-zinc-600">({excludedRows.size} dolda)</span>
            {/if}
            {#if selectedRows.size > 0}
              <span class="text-amber-400">· {selectedRows.size} markerade</span>
            {/if}
          </span>
          <div class="flex items-center gap-3">
            <label class="flex items-center gap-1.5">
              Rader per sida
              <select
                bind:value={pageSize}
                onchange={() => currentPage = 0}
                class="appearance-none bg-zinc-800 border border-zinc-700 rounded px-1.5 py-0.5 text-white cursor-pointer"
              >
                {#each pageSizes as s}
                  <option value={s}>{s}</option>
                {/each}
              </select>
            </label>
            <div class="flex items-center gap-1">
              <button
                class="px-2 py-1 rounded bg-zinc-800 hover:bg-zinc-700 disabled:opacity-30 cursor-pointer disabled:cursor-default transition-colors text-base font-bold text-zinc-300"
                disabled={currentPage === 0}
                onclick={() => currentPage--}
              >←</button>
              <span>{currentPage + 1} / {totalPages}</span>
              <button
                class="px-2 py-1 rounded bg-zinc-800 hover:bg-zinc-700 disabled:opacity-30 cursor-pointer disabled:cursor-default transition-colors text-base font-bold text-zinc-300"
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

{#if contextMenu}
  {@const rowIdx = contextMenu.rowIdx}
  <ContextMenu
    x={contextMenu.x}
    y={contextMenu.y}
    onclose={() => contextMenu = null}
    items={[
      {
        label: `Kopiera markerade som CSV (med header)`,
        action: () => copySelectedAsCSV(true),
        disabled: selectedRows.size === 0,
      },
      {
        label: `Kopiera markerade som CSV (utan header)`,
        action: () => copySelectedAsCSV(false),
        disabled: selectedRows.size === 0,
      },
      { separator: true },
      {
        label: "Ta bort denna rad",
        action: () => deleteRow(rowIdx),
      },
      { separator: true },
      {
        label: `Ta bort markerade (${selectedRows.size})`,
        action: deleteSelected,
        disabled: selectedRows.size === 0,
      },
      {
        label: `Ta bort omarkerade`,
        action: deleteUnselected,
        disabled: selectedRows.size === 0,
      },
      { separator: true },
      {
        label: `Återställ alla (${excludedRows.size} dolda)`,
        action: restoreAll,
        disabled: excludedRows.size === 0,
      },
    ]}
  />
{/if}
