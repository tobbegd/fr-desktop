<script lang="ts">
  import { invoke as tauri } from "@tauri-apps/api/core";
  import SqlEditor from "./SqlEditor.svelte";
  import ContextMenu from "./ContextMenu.svelte";
  import SnippetPanel from "./SnippetPanel.svelte";
  import ExportPanel from "./ExportPanel.svelte";
  import { loadPrefs } from "$lib/store";
  import { buildPrompt } from "$lib/aiPrompt";

  type Props = {
    dbPath: string;
    ollamaReady: boolean;
    onOpenAiSettings: () => void;
  };
  let { dbPath, ollamaReady, onOpenAiSettings }: Props = $props();

  let showSnippets = $state(false);
  let blinkSnippet = $state(false);
  let blinkTimer: ReturnType<typeof setTimeout> | null = null;
  let sqlQuery = $state("");

  // AI
  let aiQuery = $state("");
  let aiRunning = $state(false);
  let aiError = $state("");
  let schema = $state<Record<string, string[]>>({});

  async function loadSchema() {
    if (!dbPath) return;
    try {
      schema = await tauri<Record<string, string[]>>("get_schema", { dbPath });
    } catch {}
  }

  $effect(() => {
    if (dbPath) loadSchema();
  });
  function hscroll(node: HTMLElement) {
    function onWheel(e: WheelEvent) {
      if (!e.shiftKey) return;
      e.preventDefault();
      node.scrollLeft += e.deltaY !== 0 ? e.deltaY : e.deltaX;
    }
    node.addEventListener("wheel", onWheel, { passive: false });
    return { destroy() { node.removeEventListener("wheel", onWheel); } };
  }

  let running = $state(false);
  let error = $state("");
  let result = $state<{ columns: string[]; rows: unknown[][]; truncated: boolean } | null>(null);
  let pageSize = $state(200);
  let currentPage = $state(0);

  const pageSizes = [200, 400, 800];

  let selectedRows = $state(new Set<number>());
  let excludedRows = $state(new Set<number>());
  let hiddenCols = $state(new Set<string>());
  let colFilters = $state<Record<string, string>>({});
  let contextMenu = $state<{ x: number; y: number; rowIdx: number } | null>(null);

  const hasColFilters = $derived(Object.values(colFilters).some(v => v !== ""));

  const filteredRows = $derived.by(() => {
    if (!result) return [];
    return result.rows.map((row, i) => ({ row, i })).filter(({ row, i }) => {
      if (excludedRows.has(i)) return false;
      for (const [col, filter] of Object.entries(colFilters)) {
        if (!filter) continue;
        const colIdx = result.columns.indexOf(col);
        if (colIdx === -1) continue;
        const cell = row[colIdx];
        const val = cell === null || cell === undefined ? "" : String(cell).toLowerCase();
        if (!val.includes(filter.toLowerCase())) return false;
      }
      return true;
    });
  });
  const totalPages = $derived(Math.ceil(filteredRows.length / pageSize));
  const pagedRows = $derived(filteredRows.slice(currentPage * pageSize, (currentPage + 1) * pageSize));

  let sortKeys = $state<{ col: string; dir: "ASC" | "DESC" }[]>([]);

  function parseSelectCols(sql: string): string[] {
    const match = sql.match(/^\s*SELECT\s+([\s\S]+?)\s+FROM\s+/i);
    if (!match) return [];
    const raw = match[1];
    const cols: string[] = [];
    let depth = 0, start = 0;
    for (let i = 0; i < raw.length; i++) {
      if (raw[i] === "(") depth++;
      else if (raw[i] === ")") depth--;
      else if (raw[i] === "," && depth === 0) {
        cols.push(raw.slice(start, i).trim());
        start = i + 1;
      }
    }
    cols.push(raw.slice(start).trim());
    return cols;
  }

  function getColAlias(expr: string): string {
    const asMatch = expr.match(/\bAS\s+["'`]?(\w+)["'`]?\s*$/i);
    if (asMatch) return asMatch[1];
    const simple = expr.match(/["'`]?(\w+)["'`]?\s*$/);
    return simple ? simple[1] : expr;
  }

  function handleColRemove(col: string) {
    const exprs = parseSelectCols(sqlQuery);
    if (exprs.length <= 1) return;
    const remaining = exprs.filter(e => getColAlias(e) !== col);
    if (remaining.length === 0) return;
    sqlQuery = sqlQuery.replace(
      /^(\s*SELECT\s+)[\s\S]+?(\s+FROM\s+)/i,
      `$1${remaining.join(", ")}$2`
    );
    sortKeys = sortKeys.filter(k => k.col !== col);
    runQuery();
  }

  function parseSqlParts(sql: string): { base: string; limit: string } {
    let s = sql.trim();
    let limit = "";
    const limitMatch = s.match(/(\s+LIMIT\s+\d+)\s*$/i);
    if (limitMatch) {
      limit = limitMatch[1].trim();
      s = s.slice(0, s.length - limitMatch[0].length).trim();
    }
    s = s.replace(/\s+ORDER\s+BY\s+[\s\S]+$/i, "").trim();
    return { base: s, limit };
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
    const { base, limit } = parseSqlParts(sqlQuery);
    if (sortKeys.length === 0) {
      sqlQuery = limit ? `${base}\n${limit}` : base;
    } else {
      const orderBy = sortKeys.map(k => `"${k.col}" ${k.dir}`).join(", ");
      sqlQuery = `${base}\nORDER BY ${orderBy}${limit ? '\n' + limit : ''}`;
    }
    runQuery();
  }

  let lastClickedRow = $state<number | null>(null);

  function toggleRowSelect(i: number, shift: boolean) {
    const s = new Set(selectedRows);
    if (shift && lastClickedRow !== null) {
      const fromPos = filteredRows.findIndex(r => r.i === lastClickedRow);
      const toPos = filteredRows.findIndex(r => r.i === i);
      if (fromPos !== -1 && toPos !== -1) {
        const start = Math.min(fromPos, toPos);
        const end = Math.max(fromPos, toPos);
        filteredRows.slice(start, end + 1).forEach(r => s.add(r.i));
      }
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

  let exportPanel = $state<{ rows: unknown[][]; label: string } | null>(null);

  async function runAiQuery() {
    if (!aiQuery.trim()) return;
    aiRunning = true;
    aiError = "";
    try {
      const prefs = await loadPrefs();
      const model = prefs.aiModel;
      if (!model) { aiError = "Ingen AI-modell vald. Gå till Inställningar → AI-assistent."; return; }
      const schema = await tauri<Record<string, string[]>>("get_schema", { dbPath });
      const schemaText = Object.entries(schema)
        .map(([t, cols]) => `${t} (${cols.join(", ")})`)
        .join("\n");
      const sql = await tauri<string>("query_ollama", {
        model,
        prompt: buildPrompt(schema, aiQuery, model),
      });
      sqlQuery = sql.trim().replace(/^```sql\n?/i, "").replace(/```$/, "").trim();
    } catch (e) {
      aiError = String(e);
    } finally {
      aiRunning = false;
    }
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
    colFilters = {};
    hiddenCols = new Set();
    try {
      result = await tauri<{ columns: string[]; rows: unknown[][]; truncated: boolean }>(
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
    <!-- AI-fält -->
    <div class="px-3 pt-2 pb-2 border-b border-zinc-800/60">
      <div class="flex items-center gap-2">
        <span class="text-xs text-zinc-600 shrink-0">AI</span>
        <input
          type="text"
          bind:value={aiQuery}
          placeholder={ollamaReady ? "Beskriv vad du vill söka..." : "AI ej konfigurerad — klicka för att installera"}
          class="flex-1 bg-zinc-900 border rounded-md px-3 py-1.5 text-sm placeholder-zinc-600 focus:outline-none transition-colors
            {ollamaReady ? 'border-zinc-700 text-zinc-200 focus:border-zinc-500' : 'border-zinc-800 text-zinc-500 cursor-pointer'}"
          readonly={!ollamaReady}
          onfocus={() => { if (!ollamaReady) onOpenAiSettings(); }}
          onkeydown={(e) => { if (e.key === "Enter" && ollamaReady) runAiQuery(); }}
        />
        {#if ollamaReady}
          <button
            class="px-3 py-1.5 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded-md transition-colors cursor-pointer disabled:opacity-40 shrink-0"
            disabled={!aiQuery.trim() || aiRunning || !dbPath}
            onclick={runAiQuery}
          >
            {aiRunning ? "Tänker..." : "Generera"}
          </button>
        {/if}
      </div>
      {#if aiError}
        <p class="text-xs text-red-400 mt-1">{aiError}</p>
      {/if}
    </div>

    <!-- SQL-fält (alltid synligt) -->
    <div class="px-3 pt-2 pb-3 flex flex-col gap-2">
      <div class="flex items-center gap-2">
        <span class="text-xs text-zinc-600 shrink-0">SQL</span>
        <div class="flex-1"></div>
      </div>
      <div class="relative">
        <SqlEditor
          bind:value={sqlQuery}
          {schema}
          onchange={(v) => (sqlQuery = v)}
          onrun={() => runQuery(true)}
        />
        <button
          onclick={() => { showSnippets = !showSnippets; blinkSnippet = false; }}
          title={showSnippets ? "Dölj snippets" : "Visa snippets"}
          class="absolute -bottom-3 left-1/2 -translate-x-1/2 z-10 px-3 py-0.5 text-xs rounded-full border bg-zinc-900 text-zinc-500 hover:text-zinc-300 transition-colors cursor-pointer select-none
            {blinkSnippet ? 'border-orange-500 snippet-blink' : 'border-zinc-700 hover:border-zinc-500'}"
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

        <!-- Gömda kolumner -->
        {#if hiddenCols.size > 0}
          <div class="shrink-0 flex items-center gap-1 px-3 py-1 border-b border-zinc-800 flex-wrap">
            <span class="text-xs text-zinc-600 mr-1">Gömda:</span>
            {#each [...hiddenCols] as col}
              <button
                onclick={() => { hiddenCols = new Set([...hiddenCols].filter(c => c !== col)); }}
                class="px-1.5 py-0.5 text-xs rounded border border-zinc-700 bg-zinc-800/60 text-zinc-400 hover:text-white hover:border-zinc-500 cursor-pointer transition-colors"
                title="Visa kolumn"
              >{col}</button>
            {/each}
          </div>
        {/if}

        <!-- Tabell (scrollar, sticky header fungerar här) -->
        <div class="flex-1 overflow-auto min-h-0" use:hscroll>
          <table class="min-w-full text-xs text-left border-collapse">
            <thead>
              <tr>
                {#each result.columns as col}
                  {#if !hiddenCols.has(col)}
                  <th
                    class="group sticky top-0 px-3 py-2 font-medium whitespace-nowrap border-b border-zinc-800 z-10 select-none transition-colors
                      {sortKeys.some(k => k.col === col)
                        ? 'bg-zinc-800 text-white'
                        : 'bg-zinc-950 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'}"
                  >
                    <span class="flex items-center gap-1">
                      <span class="cursor-pointer" title="Sortera" onclick={() => handleHeaderClick(col)}>
                        {col}
                        {#if sortKeys.findIndex(k => k.col === col) !== -1}
                          {@const k = sortKeys.find(k => k.col === col)!}
                          {@const pos = sortKeys.findIndex(k => k.col === col) + 1}
                          <span class="ml-1 text-amber-400">
                            {k.dir === "ASC" ? "↑" : "↓"}{sortKeys.length > 1 ? pos : ""}
                          </span>
                        {/if}
                      </span>
                      <button
                        onclick={() => { hiddenCols = new Set([...hiddenCols, col]); }}
                        class="opacity-0 group-hover:opacity-60 hover:!opacity-100 text-green-500 transition-opacity cursor-pointer leading-none"
                        title="Göm kolumn"
                      >×</button>
                      {#if !sortKeys.some(k => k.col === col)}
                      <button
                        onclick={() => handleColRemove(col)}
                        class="opacity-0 group-hover:opacity-60 hover:!opacity-100 text-zinc-400 hover:text-red-400 transition-opacity cursor-pointer leading-none"
                        title="Ta bort kolumn"
                      >×</button>
                      {/if}
                    </span>
                    <input
                      type="text"
                      placeholder="filtrera…"
                      value={colFilters[col] ?? ""}
                      oninput={(e) => { colFilters = { ...colFilters, [col]: (e.target as HTMLInputElement).value }; currentPage = 0; }}
                      onclick={(e) => e.stopPropagation()}
                      class="mt-1 w-full min-w-0 px-1.5 py-0.5 text-xs font-normal rounded border
                        {colFilters[col] ? 'border-amber-600 bg-amber-950/40 text-white' : 'border-zinc-700 bg-zinc-900 text-zinc-400'}
                        placeholder-zinc-600 focus:outline-none focus:border-zinc-500 transition-colors"
                    />
                  </th>
                  {/if}
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
                  {#each result.columns as col, colIdx}
                    {#if !hiddenCols.has(col)}
                    <td class="px-3 py-1.5 whitespace-nowrap max-w-xs truncate
                      {selectedRows.has(i) ? 'text-white' : 'text-zinc-300'}">
                      {#if row[colIdx] === null}
                        <span class="text-zinc-600">NULL</span>
                      {:else}
                        {String(row[colIdx])}
                      {/if}
                    </td>
                    {/if}
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
            {#if hasColFilters}
              <button
                onclick={() => { colFilters = {}; currentPage = 0; }}
                class="ml-2 px-1.5 py-0.5 rounded bg-amber-800/50 text-amber-300 hover:bg-amber-700/50 cursor-pointer transition-colors"
              >rensa filter ×</button>
            {/if}
            {#if excludedRows.size > 0}
              <span class="text-zinc-600">({excludedRows.size} dolda)</span>
            {/if}
            {#if selectedRows.size > 0}
              <span class="text-amber-400">· {selectedRows.size} markerade</span>
            {/if}
          </span>
          <div class="flex items-center gap-3">
            {#if totalPages > 1}
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
            {/if}
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
        label: `Exportera markerade (${selectedRows.size})`,
        action: () => exportPanel = { rows: filteredRows.filter(({ i }) => selectedRows.has(i)).map(({ row }) => row), label: "markerade rader" },
        disabled: selectedRows.size === 0,
      },
      {
        label: `Exportera alla (${filteredRows.length})`,
        action: () => exportPanel = { rows: filteredRows.map(({ row }) => row), label: "alla rader" },
      },
      { separator: true },
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
      {
        label: "Avmarkera alla",
        action: () => { selectedRows = new Set(); },
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

{#if exportPanel && result}
  <ExportPanel
    rows={exportPanel.rows}
    columns={result.columns}
    label={exportPanel.label}
    onclose={() => exportPanel = null}
  />
{/if}

