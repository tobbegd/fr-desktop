<script lang="ts">
  import { invoke as tauri } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { Store } from "@tauri-apps/plugin-store";
  import SqlEditor from "./SqlEditor.svelte";
  import ContextMenu from "./ContextMenu.svelte";
  import SnippetPanel from "./SnippetPanel.svelte";
  import HistoryPanel from "./HistoryPanel.svelte";
  import SchemaPanel from "./SchemaPanel.svelte";
  import ExportPanel from "./ExportPanel.svelte";
  import NyckeltaPanel from "./NyckeltaPanel.svelte";
  import KartaPanel from "./KartaPanel.svelte";
  import { loadPrefs } from "$lib/store";
  import { debug } from "$lib/debug.svelte";
  import { buildPrompt, buildChatPrompt, type AiExpl } from "$lib/aiPrompt";
  import type { MenuItem } from "./MenuBar.svelte";

  type Props = {
    dbPath: string;
    ollamaReady: boolean;
    aiBackend: string;
    geminiApiKey: string;
    geminiModel: string;
    onOpenAiSettings: () => void;
    actionMenuItems?: MenuItem[];
    mailMenuItems?: MenuItem[];
  };
  let { dbPath, ollamaReady, aiBackend, geminiApiKey, geminiModel, onOpenAiSettings, actionMenuItems = $bindable([]), mailMenuItems = $bindable([]) }: Props = $props();

  let showSnippets = $state(false);
  let showHistory = $state(false);
  let queryHistory = $state<string[]>([]);
  let historyStore: Store | null = null;
  let blinkSnippet = $state(false);
  let showSchema = $state(false);
  let blinkTimer: ReturnType<typeof setTimeout> | null = null;
  let sqlQuery = $state("");

  // AI
  let aiQuery = $state("");
  let aiRunning = $state(false);
  let aiError = $state("");
  let aiInfo = $state("");
  let aiInfoSql = $state("");
  let aiMode = $state<"sql" | "chat">("sql");
  let aiInput = $state<HTMLInputElement | null>(null);
  const aiQueryByMode: Record<string, string> = { sql: "", chat: "" };

  function switchMode(mode: "sql" | "chat") {
    aiQueryByMode[aiMode] = aiQuery;
    aiMode = mode;
    aiQuery = aiQueryByMode[mode];
    aiInfo = "";
    aiInfoSql = "";
    setTimeout(() => { aiInput?.focus(); aiInput?.select(); }, 0);
  }
  let aiAutoExec = $state(true);
  let schema = $state<Record<string, string[]>>({});
  let aiExpl = $state<AiExpl>({});

  async function loadSchema() {
    if (!dbPath) return;
    try {
      [schema, aiExpl] = await Promise.all([
        tauri<Record<string, string[]>>("get_schema", { dbPath }),
        tauri<AiExpl>("get_ai_explanations", { dbPath }).catch(() => ({} as AiExpl)),
      ]);
    } catch {}
  }

  $effect(() => {
    if (dbPath) loadSchema();
  });

  async function loadHistory() {
    historyStore = await Store.load("history.json");
    queryHistory = (await historyStore.get<string[]>("queries")) ?? [];
  }

  async function addToHistory(sql: string) {
    const trimmed = sql.trim();
    if (!trimmed || queryHistory.includes(trimmed)) return;
    queryHistory = [trimmed, ...queryHistory].slice(0, 10);
    await historyStore?.set("queries", queryHistory);
    await historyStore?.save();
  }

  async function removeFromHistory(sql: string) {
    queryHistory = queryHistory.filter(q => q !== sql);
    await historyStore?.set("queries", queryHistory);
    await historyStore?.save();
  }

  $effect(() => { loadHistory(); });
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
  let contextMenu = $state<{ x: number; y: number; rowIdx: number; cellValue?: string; url?: string } | null>(null);
  let nyckeltaPanel = $state<{ orgnr: string; orgnamn: string } | null>(null);
  let kartaPanel = $state<{ orgnr: string; orgnamn: string; lat: number | null; lon: number | null; postort_lat: number | null; postort_lon: number | null; postort: string | null; gatuadress: string | null }[] | null>(null);
  let kartaSearch = $state(false);

  function onKartaSearchResult(orgnrs: string[]) {
    kartaSearch = false;
    if (orgnrs.length === 0) return;
    const inList = orgnrs.map(o => `'${o.replace(/'/g, "''")}'`).join(", ");
    sqlQuery = `SELECT orgnr, orgnamn, gatuadress, postnummer, postort, lat, lon\nFROM bolag\nWHERE orgnr IN (${inList})`;
    runQuery(true);
  }

  function getKartaRows(rowIndices: number[]) {
    if (!result) return [];
    const ci = (col: string) => result!.columns.indexOf(col);
    return rowIndices.map(i => {
      const r = result!.rows[i];
      const n = (col: string) => { const v = r[ci(col)]; return (v !== null && v !== undefined && v !== "") ? Number(v) : null; };
      const s = (col: string) => { const v = r[ci(col)]; return (v !== null && v !== undefined && v !== "") ? String(v) : null; };
      return {
        orgnr: s("orgnr") ?? "",
        orgnamn: s("orgnamn") ?? s("orgnr") ?? "",
        lat: n("lat"), lon: n("lon"),
        postort_lat: n("postort_lat"), postort_lon: n("postort_lon"),
        postort: s("postort"), gatuadress: s("gatuadress"),
      };
    }).filter(b => (b.lat !== null && b.lon !== null) || (b.postort_lat !== null && b.postort_lon !== null));
  }
  let hoveredRow = $state<number | null>(null);

  const ROW_INDICATORS = [
    { key: "ar_year",    label: "Nyckeltal", icon: "📊" },
    { key: "webbadress", label: "Webb",      icon: "🌐" },
    { key: "email",      label: "E-post",    icon: "✉️" },
    { key: "telefon",    label: "Telefon",   icon: "📞" },
    { key: "lon",        label: "Karta",     icon: "📍" },
  ] as const;

  const hoveredIndicators = $derived.by(() => {
    if (hoveredRow === null || !result) return null;
    const row = result.rows[hoveredRow];
    if (!row) return null;
    return ROW_INDICATORS.map(ind => {
      const idx = result!.columns.indexOf(ind.key);
      const val = idx !== -1 ? row[idx] : null;
      return { ...ind, active: val !== null && val !== undefined && val !== "" };
    });
  });

  const hasColFilters = $derived(Object.values(colFilters).some(v => v !== ""));

  const filteredRows = $derived.by(() => {
    if (!result) return [];
    return result.rows.map((row, i) => ({ row, i })).filter(({ row, i }) => {
      if (excludedRows.has(i)) return false;
      for (const [col, filter] of Object.entries(colFilters)) {
        if (!filter) continue;
        const colIdx = result!.columns.indexOf(col);
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
  let copiedFeedback = $state("");
  let copiedTimer: ReturnType<typeof setTimeout> | null = null;

  function flashCopied(val: string) {
    copiedFeedback = `Kopierade "${val.length > 30 ? val.slice(0, 30) + "…" : val}"`;
    if (copiedTimer) clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => { copiedFeedback = ""; }, 2000);
  }

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

  function openContextMenu(e: MouseEvent, rowIdx: number, cellValue?: string) {
    e.preventDefault();
    let url: string | undefined;
    if (result) {
      const urlColIdx = result.columns.indexOf("webbadress");
      if (urlColIdx !== -1) {
        const raw = result.rows[rowIdx]?.[urlColIdx];
        if (raw) {
          url = String(raw);
          if (!/^https?:\/\//i.test(url)) url = "https://" + url;
        }
      }
    }
    contextMenu = { x: e.clientX, y: e.clientY, rowIdx, cellValue, url };
  }

  function getRowMeta(rowIdx: number): { orgnr: string; orgnamn: string } | null {
    if (!result) return null;
    const orgnrIdx = result.columns.indexOf("orgnr");
    const orgnamnIdx = result.columns.indexOf("orgnamn");
    if (orgnrIdx === -1) return null;
    const orgnr = result.rows[rowIdx]?.[orgnrIdx];
    const orgnamn = orgnamnIdx !== -1 ? result.rows[rowIdx]?.[orgnamnIdx] : null;
    if (!orgnr) return null;
    return { orgnr: String(orgnr), orgnamn: orgnamn ? String(orgnamn) : String(orgnr) };
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

  // Säck-dialog
  type SackItem = { id: number; namn: string; antal: number };
  let sackDialog = $state<"ny" | "befintlig" | null>(null);
  let sackNamn = $state("");
  let sackarList = $state<SackItem[]>([]);
  let sackBusy = $state(false);
  let sackError = $state("");
  let sackResult = $state<{ tillagda: number; ignorerade: number; duplikater: number } | null>(null);

  function getRowMailData(rowIdx: number): { orgnr: string; orgnamn: string; email: string; reklamsparr: string } | null {
    if (!result) return null;
    const orgnrIdx = result.columns.indexOf("orgnr");
    if (orgnrIdx === -1) return null;
    const orgnrVal = result.rows[rowIdx]?.[orgnrIdx];
    if (!orgnrVal) return null;
    const orgnamnIdx = result.columns.indexOf("orgnamn");
    const emailIdx = result.columns.indexOf("email");
    const reklamsparrIdx = result.columns.indexOf("reklamsparr");
    return {
      orgnr: String(orgnrVal),
      orgnamn: orgnamnIdx !== -1 && result.rows[rowIdx]?.[orgnamnIdx] ? String(result.rows[rowIdx][orgnamnIdx]) : String(orgnrVal),
      email: emailIdx !== -1 && result.rows[rowIdx]?.[emailIdx] ? String(result.rows[rowIdx][emailIdx]) : "",
      reklamsparr: reklamsparrIdx !== -1 && result.rows[rowIdx]?.[reklamsparrIdx] ? String(result.rows[rowIdx][reklamsparrIdx]) : "",
    };
  }

  function getSelectedMailData() {
    return [...selectedRows].map(i => getRowMailData(i)).filter((d): d is NonNullable<typeof d> => d !== null);
  }

  async function oppnaSackDialog(typ: "ny" | "befintlig") {
    sackError = "";
    sackNamn = "";
    sackResult = null;
    if (typ === "befintlig") {
      sackarList = await tauri<SackItem[]>("list_sackar");
    }
    sackDialog = typ;
  }

  async function skapaSackFranMarkerade() {
    const namn = sackNamn.trim();
    if (!namn) return;
    sackBusy = true;
    sackError = "";
    sackResult = null;
    try {
      const alla = getSelectedMailData();
      const medEmail = alla.filter(d => d.email.trim() !== "");
      // Deduplicera på orgnr (ifall samma bolag förekommer flera gånger i resultatet)
      const seen = new Set<string>();
      const unika = medEmail.filter(d => { if (seen.has(d.orgnr)) return false; seen.add(d.orgnr); return true; });
      const id = await tauri<number>("create_sack", { namn });
      for (const d of unika) {
        await tauri("add_bolag_to_sack", { sackId: id, orgnr: d.orgnr, orgnamn: d.orgnamn, email: d.email, reklamsparr: d.reklamsparr });
      }
      sackResult = { tillagda: unika.length, ignorerade: alla.length - medEmail.length, duplikater: medEmail.length - unika.length };
      sackNamn = "";
    } catch (e) {
      sackError = String(e);
    } finally {
      sackBusy = false;
    }
  }

  async function laggTillISack(sackId: number) {
    sackBusy = true;
    sackError = "";
    sackResult = null;
    try {
      const alla = getSelectedMailData();
      const medEmail = alla.filter(d => d.email.trim() !== "");
      // Hämta befintliga orgnr i säcken för dubblettcheck
      const befintliga = await tauri<{ orgnr: string }[]>("list_sack_bolag", { sackId });
      const befintligaSet = new Set(befintliga.map(b => b.orgnr));
      const seen = new Set<string>();
      const nya = medEmail.filter(d => {
        if (befintligaSet.has(d.orgnr) || seen.has(d.orgnr)) return false;
        seen.add(d.orgnr);
        return true;
      });
      for (const d of nya) {
        await tauri("add_bolag_to_sack", { sackId, orgnr: d.orgnr, orgnamn: d.orgnamn, email: d.email, reklamsparr: d.reklamsparr });
      }
      sackResult = { tillagda: nya.length, ignorerade: alla.length - medEmail.length, duplikater: medEmail.length - nya.length };
    } catch (e) {
      sackError = String(e);
    } finally {
      sackBusy = false;
    }
  }

  function fixSql(sql: string): string {
    // ILIKE doesn't exist in SQLite
    let fixed = sql.replace(/(\w+)\s+ILIKE\s+('[^']*')/gi, "LOWER($1) LIKE LOWER($2)");
    // Normalize all postort comparisons to ulow() which handles Swedish chars correctly
    fixed = fixed.replace(/LOWER\s*\(\s*postort\s*\)\s+LIKE\s+LOWER\s*\(\s*'([^']*)'\s*\)/gi,
      (_, term) => `ulow(postort) LIKE '${term.toLowerCase()}'`);
    fixed = fixed.replace(/postort\s*=\s*'([^']*)'/gi,
      (_, term) => `ulow(postort) LIKE '%${term.toLowerCase()}%'`);
    fixed = fixed.replace(/postort\s+LIKE\s+'([^']*)'/gi,
      (_, term) => `ulow(postort) LIKE '${term.toLowerCase()}'`);
    return fixed;
  }

  function extractSqlFromText(text: string): string {
    // Try ```sql ... ``` block first
    const fenced = text.match(/```sql\s*([\s\S]*?)```/i);
    if (fenced) return fenced[1].trim();
    // Try plain ``` ... ``` block
    const plain = text.match(/```\s*(SELECT|WITH|PRAGMA|INSERT|UPDATE|DELETE)[\s\S]*?```/i);
    if (plain) return plain[0].replace(/^```\w*\s*/i, "").replace(/```$/, "").trim();
    // Try inline `...` containing SQL
    const inline = text.match(/`((?:SELECT|WITH|PRAGMA|INSERT|UPDATE|DELETE)[\s\S]*?)`/i);
    if (inline) return inline[1].trim();
    // Try a line starting with a SQL keyword
    const lines = text.split("\n");
    const start = lines.findIndex(l => /^\s*(SELECT|WITH|PRAGMA|INSERT|UPDATE|DELETE)\b/i.test(l));
    if (start !== -1) return lines.slice(start).join("\n").trim();
    return "";
  }

  function looksLikeSql(text: string): boolean {
    const t = text.trimStart().toUpperCase();
    return /^(SELECT|INSERT|UPDATE|DELETE|WITH|PRAGMA|CREATE|DROP|ALTER|EXPLAIN|ATTACH|DETACH|BEGIN|COMMIT|ROLLBACK)[\s(]/.test(t);
  }

  async function callAi(prompt: string): Promise<string> {
    const backendLabel = aiBackend === "gemini" ? `Gemini (${geminiModel})` : "Ollama";
    if (debug.ai) debug.log(`→ ${backendLabel}\n${prompt}`);
    let result: string;
    if (aiBackend === "gemini") {
      if (!geminiApiKey) throw new Error("Ingen Gemini API-nyckel. Gå till Inställningar → AI-assistent.");
      result = await tauri<string>("query_gemini", { apiKey: geminiApiKey, model: geminiModel, prompt });
    } else {
      const prefs = await loadPrefs();
      const model = prefs.aiModel;
      if (!model) throw new Error("Ingen Ollama-modell vald. Gå till Inställningar → AI-assistent.");
      result = await tauri<string>("query_ollama", { model, prompt });
    }
    if (debug.ai) debug.log(`← ${backendLabel}\n${result}`);
    return result;
  }

  async function runAiQuery() {
    if (!aiQuery.trim()) return;
    aiRunning = true;
    aiError = "";
    aiInfo = "";
    aiInfoSql = "";
    try {
      const schema = await tauri<Record<string, string[]>>("get_schema", { dbPath });
      if (aiMode === "chat") {
        const raw = await callAi(buildChatPrompt(schema, aiQuery, aiExpl, sqlQuery));
        aiInfo = raw.trim();
        aiInfoSql = extractSqlFromText(aiInfo);
      } else {
        const raw = await callAi(buildPrompt(schema, aiQuery, aiExpl));
        const rawTrimmed = raw.trim();
        let candidate = rawTrimmed.replace(/^```sql\n?/i, "").replace(/```$/, "").trim();
        if (!looksLikeSql(candidate)) {
          candidate = extractSqlFromText(rawTrimmed) || candidate;
        }
        const cleaned = fixSql(candidate);
        if (looksLikeSql(cleaned)) {
          sqlQuery = cleaned;
          if (aiAutoExec) runQuery(true);
        } else {
          aiError = "AI returnerade inte giltig SQL. Försök igen eller byt till chat-läge.";
        }
      }
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
      addToHistory(sqlQuery);
    } catch (e) {
      error = String(e);
    } finally {
      running = false;
    }
  }

  $effect(() => {
    const items: MenuItem[] = [];
    const sel = [...selectedRows];

    // Nyckeltal – kräver exakt 1 markerad rad med ar_year
    if (sel.length === 1 && result) {
      const meta = getRowMeta(sel[0]);
      const arYearIdx = result.columns.indexOf("ar_year");
      const hasAr = arYearIdx !== -1 && result.rows[sel[0]]?.[arYearIdx] != null && result.rows[sel[0]]?.[arYearIdx] !== "";
      const name = meta ? (meta.orgnamn.length > 30 ? meta.orgnamn.slice(0, 30) + "…" : meta.orgnamn) : "";
      items.push(hasAr && meta
        ? { label: `Visa nyckeltal: ${name}`, action: () => { nyckeltaPanel = meta; } }
        : { label: "Visa nyckeltal", action: () => {}, disabled: true });
    } else {
      items.push({ label: "Visa nyckeltal", action: () => {}, disabled: true });
    }

    // Webbplats – kräver exakt 1 markerad rad med webbadress
    if (sel.length === 1 && result) {
      const urlColIdx = result.columns.indexOf("webbadress");
      const raw = urlColIdx !== -1 ? result.rows[sel[0]]?.[urlColIdx] : null;
      let url = raw ? String(raw) : null;
      if (url && !/^https?:\/\//i.test(url)) url = "https://" + url;
      items.push(url
        ? { label: "Öppna webbplats", action: () => openUrl(url!) }
        : { label: "Öppna webbplats", action: () => {}, disabled: true });
    } else {
      items.push({ label: "Öppna webbplats", action: () => {}, disabled: true });
    }

    // Karta
    const kartaRowIndices = sel.length > 0 ? sel : (result ? result.rows.map((_, i) => i) : []);
    const kartaRows = getKartaRows(kartaRowIndices);
    const kartaLabel = sel.length > 1 ? `Visa ${sel.length} markerade på karta` : "Visa på karta";
    items.push(kartaRows.length > 0
      ? { label: kartaLabel, action: () => { kartaPanel = kartaRows; } }
      : { label: "Visa på karta", action: () => {}, disabled: true });

    items.push({ separator: true });

    items.push({
      label: `Exportera markerade (${selectedRows.size})`,
      action: () => { exportPanel = { rows: filteredRows.filter(({ i }) => selectedRows.has(i)).map(({ row }) => row), label: "markerade rader" }; },
      disabled: selectedRows.size === 0,
    });
    items.push({
      label: `Exportera alla (${filteredRows.length})`,
      action: () => { exportPanel = { rows: filteredRows.map(({ row }) => row), label: "alla rader" }; },
      disabled: filteredRows.length === 0,
    });

    items.push({ separator: true });

    items.push({
      label: "Kopiera markerade som CSV (med header)",
      action: () => copySelectedAsCSV(true),
      disabled: selectedRows.size === 0,
    });
    items.push({
      label: "Kopiera markerade som CSV (utan header)",
      action: () => copySelectedAsCSV(false),
      disabled: selectedRows.size === 0,
    });
    items.push({
      label: "Avmarkera alla",
      action: () => { selectedRows = new Set(); },
      disabled: selectedRows.size === 0,
    });

    items.push({ separator: true });

    items.push({
      label: `Ta bort markerade (${selectedRows.size})`,
      action: deleteSelected,
      disabled: selectedRows.size === 0,
    });
    items.push({
      label: "Ta bort omarkerade",
      action: deleteUnselected,
      disabled: selectedRows.size === 0,
    });

    items.push({ separator: true });

    items.push({
      label: `Återställ alla (${excludedRows.size} dolda)`,
      action: restoreAll,
      disabled: excludedRows.size === 0,
    });

    actionMenuItems = items;

    const harOrgnr = result !== null && result.columns.includes("orgnr");
    mailMenuItems = [
      {
        label: selectedRows.size > 0 ? `Skapa brevsäck från markerade (${selectedRows.size})` : "Skapa brevsäck från markerade",
        action: () => oppnaSackDialog("ny"),
        disabled: !harOrgnr || selectedRows.size === 0,
      },
      {
        label: "Lägg till i brevsäck",
        action: () => oppnaSackDialog("befintlig"),
        disabled: !harOrgnr || selectedRows.size === 0,
      },
    ];
  });

</script>

<!-- SearchArea fyller återstående höjd i föräldern -->
<div class="flex-1 flex flex-col min-h-0">

  <!-- Sök-input (fast höjd) -->
  <div class="shrink-0 border-b border-zinc-800">
    <!-- AI-fält -->
    <div class="px-3 pt-2 pb-2 border-b border-zinc-800/60">
      <div class="flex items-center gap-2">
        <span class="text-xs text-zinc-600 shrink-0">AI</span>
        <div class="flex rounded-md overflow-hidden border border-zinc-700 shrink-0 text-xs">
          <button
            class="px-2 py-1 transition-colors cursor-pointer {aiMode === 'sql' ? 'bg-zinc-700 text-zinc-200' : 'bg-zinc-900 text-zinc-500 hover:text-zinc-300'}"
            onclick={() => switchMode('sql')}
          >SQL</button>
          <button
            class="px-2 py-1 transition-colors cursor-pointer {aiMode === 'chat' ? 'bg-zinc-700 text-zinc-200' : 'bg-zinc-900 text-zinc-500 hover:text-zinc-300'}"
            onclick={() => switchMode('chat')}
          >Chat</button>
        </div>
        <input
          type="text"
          bind:this={aiInput}
          bind:value={aiQuery}
          placeholder={ollamaReady ? (aiMode === 'chat' ? "Ställ en fråga..." : "Beskriv vad du vill söka...") : "AI ej konfigurerad — klicka för att installera"}
          class="flex-1 bg-zinc-900 border rounded-md px-3 py-1.5 text-sm placeholder-zinc-600 focus:outline-none transition-colors
            {ollamaReady ? 'border-zinc-700 text-zinc-200 focus:border-zinc-500' : 'border-zinc-800 text-zinc-500 cursor-pointer'}"
          readonly={!ollamaReady}
          onfocus={() => { if (!ollamaReady) onOpenAiSettings(); }}
          onkeydown={(e) => { if (e.key === "Enter" && ollamaReady) runAiQuery(); }}
          oninput={(e) => { if (!(e.target as HTMLInputElement).value) aiInfo = ""; }}
        />
        {#if ollamaReady}
          <button
            class="px-3 py-1.5 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded-md transition-colors cursor-pointer disabled:opacity-40 shrink-0"
            disabled={!aiQuery.trim() || aiRunning || !dbPath}
            onclick={runAiQuery}
          >
            {#if aiRunning}
              <span class="flex items-center gap-1.5">
                <span class="flex gap-0.5">
                  <span class="w-1 h-1 rounded-full bg-rose-400 animate-bounce [animation-delay:0ms]"></span>
                  <span class="w-1 h-1 rounded-full bg-amber-400 animate-bounce [animation-delay:150ms]"></span>
                  <span class="w-1 h-1 rounded-full bg-emerald-400 animate-bounce [animation-delay:300ms]"></span>
                </span>
                Tänker
              </span>
            {:else}
              Generera
            {/if}
          </button>
          {#if aiMode === 'sql'}
          <label class="flex items-center gap-1.5 cursor-pointer shrink-0 select-none">
            <input type="checkbox" bind:checked={aiAutoExec} class="accent-zinc-400 cursor-pointer" />
            <span class="text-xs text-zinc-500">auto</span>
          </label>
          {/if}
        {/if}
      </div>
      {#if aiError}
        <p class="text-xs text-red-400 mt-1">{aiError}</p>
      {/if}
      {#if aiInfo}
        <div class="relative mt-1 mb-3">
          <div class="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 max-h-48 overflow-y-auto">
            <p class="text-sm text-zinc-300 whitespace-pre-wrap leading-relaxed">{aiInfo}</p>
          </div>
          <div class="absolute -bottom-3 left-1/2 -translate-x-1/2 z-10 flex items-center gap-2">
            {#if aiInfoSql}
              <button
                onclick={() => { sqlQuery = fixSql(aiInfoSql); aiInfo = ""; aiInfoSql = ""; runQuery(true); }}
                class="px-3 py-0.5 text-xs rounded-full border border-zinc-600 hover:border-emerald-500 bg-zinc-900 text-zinc-400 hover:text-emerald-400 transition-colors cursor-pointer select-none"
              >Kör AI:s förslag</button>
            {/if}
            <button
              onclick={() => { aiInfo = ""; aiInfoSql = ""; }}
              title="Stäng"
              class="px-3 py-0.5 text-xs rounded-full border border-zinc-700 hover:border-zinc-500 bg-zinc-900 text-zinc-500 hover:text-zinc-300 transition-colors cursor-pointer select-none"
            >▲</button>
          </div>
        </div>
      {/if}
    </div>

    <!-- SQL-fält (alltid synligt) -->
    <div class="px-3 pt-2 pb-3 flex flex-col gap-2">
      <div class="flex items-center gap-2">
        <span class="text-xs text-zinc-600 shrink-0">SQL</span>
        <div class="flex-1"></div>
        <div class="flex items-center gap-2">
          {#each ROW_INDICATORS as ind}
            {@const active = hoveredIndicators?.find(h => h.key === ind.key)?.active ?? false}
            <span class="relative group">
              <span
                class="text-sm transition-all duration-150 select-none cursor-default"
                class:opacity-10={!hoveredIndicators}
                class:opacity-15={hoveredIndicators && !active}
                class:opacity-100={active}
              >{ind.icon}</span>
              <span class="absolute bottom-full left-1/2 -translate-x-1/2 mb-1 px-1.5 py-0.5 text-xs rounded bg-zinc-800 text-zinc-300 whitespace-nowrap opacity-0 group-hover:opacity-100 pointer-events-none transition-opacity">{ind.label}</span>
            </span>
          {/each}
        </div>
      </div>
      <div class="relative">
        <SqlEditor
          bind:value={sqlQuery}
          {schema}
          onchange={(v) => (sqlQuery = v)}
          onrun={() => runQuery(true)}
        />
        <button
          onclick={() => { showSnippets = !showSnippets; showHistory = false; blinkSnippet = false; }}
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
      <div class="relative flex items-center justify-between">
        <div class="flex items-center gap-3">
          <span class="text-xs text-zinc-600">Ctrl+Enter för att köra</span>
          <button
            onclick={() => { showSchema = !showSchema; showHistory = false; }}
            class="text-xs transition-colors cursor-pointer select-none {showSchema ? 'text-zinc-200 hover:text-white' : 'text-zinc-600 hover:text-zinc-300'}"
          >{showSchema ? "Dölj schema" : "Visa schema"}</button>
          <button
            onclick={() => { showHistory = !showHistory; showSchema = false; }}
            class="text-xs transition-colors cursor-pointer select-none {showHistory ? 'text-zinc-200 hover:text-white' : 'text-zinc-600 hover:text-zinc-300'}"
          >{showHistory ? "Dölj historik" : "Historik"}</button>
          <button
            onclick={() => { kartaSearch = true; }}
            disabled={!dbPath}
            class="text-xs text-zinc-600 hover:text-zinc-300 transition-colors cursor-pointer select-none disabled:opacity-30"
          >Kartasökning</button>
        </div>
        <button
          class="px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded-md hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-40"
          disabled={!sqlQuery.trim() || running || !dbPath}
          onclick={() => runQuery(true)}
        >
          {running ? "Kör..." : "Kör"}
        </button>
        {#if showHistory}
          <div class="absolute top-full left-0 z-20 mt-1 w-full max-h-64 overflow-y-auto rounded-lg border border-zinc-700 bg-zinc-900 shadow-xl">
            <HistoryPanel
              items={queryHistory}
              onselect={(sql) => { sqlQuery = sql; showHistory = false; runQuery(true); }}
              onremove={removeFromHistory}
            />
          </div>
        {/if}
        {#if showSchema}
          <div class="absolute top-full left-0 z-20 mt-1 w-full max-h-64 overflow-y-auto rounded-lg border border-zinc-700 bg-zinc-900 shadow-xl">
            <SchemaPanel {dbPath} />
          </div>
        {/if}
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
                  onmouseenter={() => hoveredRow = i}
                  onmouseleave={() => hoveredRow = null}
                >
                  {#each result.columns as col, colIdx}
                    {#if !hiddenCols.has(col)}
                    <td class="px-3 py-1.5 whitespace-nowrap max-w-xs truncate
                      {selectedRows.has(i) ? 'text-white' : 'text-zinc-300'}"
                      oncontextmenu={(e) => { e.stopPropagation(); openContextMenu(e, i, row[colIdx] !== null && row[colIdx] !== undefined ? String(row[colIdx]) : undefined); }}
                      ondblclick={(e) => { e.stopPropagation(); if (row[colIdx] !== null && row[colIdx] !== undefined) { const v = String(row[colIdx]); navigator.clipboard.writeText(v); flashCopied(v); } }}
                    >
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
            {#if copiedFeedback}
              <span class="text-green-400">· {copiedFeedback}</span>
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

{#if kartaPanel}
  <KartaPanel bolag={kartaPanel} onclose={() => kartaPanel = null} />
{/if}

{#if kartaSearch}
  <KartaPanel
    bolag={[]}
    searchMode={true}
    {dbPath}
    onsearchresult={onKartaSearchResult}
    onclose={() => kartaSearch = false}
  />
{/if}

{#if nyckeltaPanel}
  <NyckeltaPanel
    {dbPath}
    orgnr={nyckeltaPanel.orgnr}
    orgnamn={nyckeltaPanel.orgnamn}
    onclose={() => nyckeltaPanel = null}
  />
{/if}

{#if contextMenu}
  {@const rowIdx = contextMenu.rowIdx}
  <ContextMenu
    x={contextMenu.x}
    y={contextMenu.y}
    onclose={() => contextMenu = null}
    items={[
      contextMenu.cellValue !== undefined
        ? { label: `Kopiera "${contextMenu.cellValue.length > 30 ? contextMenu.cellValue.slice(0, 30) + "…" : contextMenu.cellValue}" (dubbelklicka på cell)`, action: () => navigator.clipboard.writeText(contextMenu!.cellValue ?? "") }
        : { label: "Dubbelklicka på en cell för att kopiera värdet", action: () => {}, disabled: true },
      contextMenu.url
        ? { label: `Öppna webbplats: ${contextMenu.url.length > 40 ? contextMenu.url.slice(0, 40) + "…" : contextMenu.url}`, action: () => openUrl(contextMenu!.url!) }
        : { label: "Ingen webbadress för denna rad", action: () => {}, disabled: true },
      (() => {
        const meta = getRowMeta(rowIdx);
        if (!meta) return { label: "Ingen orgnr på denna rad", action: () => {}, disabled: true };
        const arYearIdx = result?.columns.indexOf("ar_year") ?? -1;
        const hasArData = arYearIdx !== -1 && result?.rows[rowIdx]?.[arYearIdx] !== null && result?.rows[rowIdx]?.[arYearIdx] !== undefined && result?.rows[rowIdx]?.[arYearIdx] !== "";
        const name = meta.orgnamn.length > 35 ? meta.orgnamn.slice(0, 35) + "…" : meta.orgnamn;
        return hasArData
          ? { label: `Visa nyckeltal: ${name}`, action: () => { nyckeltaPanel = meta; } }
          : { label: `Visa nyckeltal: ${name}`, action: () => {}, disabled: true };
      })(),
      (() => {
        const rows = getKartaRows(selectedRows.size > 0 ? [...selectedRows] : [rowIdx]);
        const label = selectedRows.size > 1 ? `Visa ${selectedRows.size} markerade på karta` : "Visa på karta";
        return rows.length > 0
          ? { label, action: () => { kartaPanel = rows; } }
          : { label: "Ingen kartdata på dessa rader", action: () => {}, disabled: true };
      })(),
      { separator: true },
      {
        label: `Exportera markerade (${selectedRows.size})`,
        action: () => exportPanel = { rows: filteredRows.filter(({ i }) => selectedRows.has(i)).map(({ row }) => row), label: "markerade rader" },
        disabled: selectedRows.size === 0,
      },
      {
        label: `Exportera alla (${filteredRows.length})`,
        action: () => exportPanel = { rows: filteredRows.map(({ row }) => row), label: "alla rader" },
        disabled: filteredRows.length === 0,
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

{#if sackDialog}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
    role="dialog"
    aria-modal="true"
    onclick={() => { sackDialog = null; }}
  >
    <div
      class="bg-zinc-900 border border-zinc-700 rounded-xl p-5 w-80 shadow-2xl"
      onclick={(e) => e.stopPropagation()}
    >
      {#if sackResult}
        <p class="text-sm text-zinc-200 mb-3">
          {sackResult.tillagda} bolag {sackDialog === "ny" ? "skapade i ny säck" : "tillagda"}.
        </p>
        {#if sackResult.ignorerade > 0 || sackResult.duplikater > 0}
          <div class="flex flex-col gap-1 mb-3">
            {#if sackResult.ignorerade > 0}
              <p class="text-xs text-zinc-400">{sackResult.ignorerade} ignorerades — e-post saknas.</p>
              <p class="text-xs text-zinc-600">Tips: sök med "med email" i AI-söket.</p>
            {/if}
            {#if sackResult.duplikater > 0}
              <p class="text-xs text-zinc-400 {sackResult.ignorerade > 0 ? 'mt-1' : ''}">{sackResult.duplikater} hoppades över — finns redan i säcken.</p>
            {/if}
          </div>
        {/if}
        <div class="flex justify-end">
          <button onclick={() => { sackDialog = null; sackResult = null; }} class="px-3 py-1.5 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded cursor-pointer transition-colors">Stäng</button>
        </div>
      {:else if sackDialog === "ny"}
        <h2 class="text-sm font-medium text-zinc-200 mb-1">Skapa brevsäck</h2>
        <p class="text-xs text-zinc-500 mb-3">{selectedRows.size} markerade — bolag utan e-post ignoreras.</p>
        <input
          bind:value={sackNamn}
          placeholder="Namn på säcken..."
          autofocus
          onkeydown={(e) => e.key === "Enter" && skapaSackFranMarkerade()}
          class="w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-500 mb-3"
        />
        {#if sackError}<p class="text-xs text-red-400 mb-2">{sackError}</p>{/if}
        <div class="flex justify-end gap-2">
          <button onclick={() => sackDialog = null} class="px-3 py-1.5 text-xs text-zinc-400 hover:text-white cursor-pointer transition-colors">Avbryt</button>
          <button
            onclick={skapaSackFranMarkerade}
            disabled={sackBusy || !sackNamn.trim()}
            class="px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded hover:bg-zinc-200 cursor-pointer disabled:opacity-50 transition-colors"
          >{sackBusy ? "..." : "Skapa"}</button>
        </div>
      {:else}
        <h2 class="text-sm font-medium text-zinc-200 mb-1">Lägg till i brevsäck</h2>
        <p class="text-xs text-zinc-500 mb-3">{selectedRows.size} markerade — bolag utan e-post ignoreras.</p>
        {#if sackarList.length === 0}
          <p class="text-xs text-zinc-500 mb-3">Inga säckar finns. Skapa en under Utskick → Säckar.</p>
        {:else}
          <div class="flex flex-col gap-1 max-h-52 overflow-y-auto mb-3">
            {#each sackarList as s}
              <button
                onclick={() => laggTillISack(s.id)}
                disabled={sackBusy}
                class="text-left px-3 py-2 rounded text-sm text-zinc-300 hover:bg-zinc-800 hover:text-white transition-colors cursor-pointer disabled:opacity-50"
              >{s.namn} <span class="text-zinc-600 text-xs">({s.antal} bolag)</span></button>
            {/each}
          </div>
        {/if}
        {#if sackError}<p class="text-xs text-red-400 mb-2">{sackError}</p>{/if}
        <button onclick={() => sackDialog = null} class="px-3 py-1.5 text-xs text-zinc-400 hover:text-white cursor-pointer transition-colors">Stäng</button>
      {/if}
    </div>
  </div>
{/if}

