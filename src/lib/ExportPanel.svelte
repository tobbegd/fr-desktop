<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import * as XLSX from "xlsx";

  type Props = {
    rows: unknown[][];
    columns: string[];
    label: string;
    onclose: () => void;
  };

  let { rows, columns, label, onclose }: Props = $props();

  type Format = "csv" | "xlsx" | "json";
  let selectedFormats = $state<Set<Format>>(new Set(["csv"]));
  let exportError = $state("");

  const formats: { id: Format; label: string }[] = [
    { id: "csv",  label: "CSV" },
    { id: "xlsx", label: "Excel" },
    { id: "json", label: "JSON" },
  ];

  function toggleFormat(id: Format) {
    const s = new Set(selectedFormats);
    s.has(id) ? s.delete(id) : s.add(id);
    selectedFormats = s;
  }

  function buildCsv(): string {
    function escapeCell(val: unknown): string {
      if (val === null || val === undefined) return "";
      const s = String(val);
      return s.includes(",") || s.includes('"') || s.includes("\n")
        ? `"${s.replace(/"/g, '""')}"`
        : s;
    }
    const lines = [columns.map(escapeCell).join(",")];
    for (const row of rows) lines.push(row.map(escapeCell).join(","));
    return "\uFEFF" + lines.join("\n");
  }

  function buildXlsx(): string {
    const data = [columns, ...rows.map(r => r.map(c => c === null || c === undefined ? "" : c))];
    const ws = XLSX.utils.aoa_to_sheet(data);
    const wb = XLSX.utils.book_new();
    XLSX.utils.book_append_sheet(wb, ws, "Export");
    return XLSX.write(wb, { type: "base64", bookType: "xlsx" });
  }

  function buildJson(): string {
    const data = rows.map(row =>
      Object.fromEntries(columns.map((col, i) => [col, row[i] ?? null]))
    );
    return JSON.stringify(data, null, 2);
  }

  async function doExport() {
    exportError = "";
    try {
      for (const fmt of selectedFormats) {
        if (fmt === "csv") {
          const content = buildCsv();
          await invoke("save_file", { filename: "export.csv", content, extension: "csv" });
        } else if (fmt === "xlsx") {
          const b64 = buildXlsx();
          await invoke("save_file_binary", { filename: "export.xlsx", data: b64 });
        } else if (fmt === "json") {
          const content = buildJson();
          await invoke("save_file", { filename: "export.json", content, extension: "json" });
        }
      }
      onclose();
    } catch (e) {
      if (String(e) !== "Avbruten") exportError = String(e);
    }
  }

  function onBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onclose();
  }
</script>

<!-- Backdrop -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
  onclick={onBackdropClick}
>
  <div class="bg-zinc-900 border border-zinc-700 rounded-xl shadow-2xl w-96 p-6 flex flex-col gap-5">

    <div class="flex items-center justify-between">
      <h2 class="text-sm font-semibold text-white">Exportera</h2>
      <button onclick={onclose} class="text-zinc-500 hover:text-white transition-colors cursor-pointer text-lg leading-none">×</button>
    </div>

    <p class="text-xs text-zinc-400">{rows.length} rader · {columns.length} kolumner · {label}</p>

    <!-- Format-toggles -->
    <div class="flex gap-2">
      {#each formats as fmt}
        <button
          onclick={() => toggleFormat(fmt.id)}
          class="flex-1 py-2 rounded-lg text-sm font-medium border transition-colors cursor-pointer
            {selectedFormats.has(fmt.id)
              ? 'bg-amber-600 border-amber-500 text-white'
              : 'bg-zinc-800 border-zinc-700 text-zinc-400 hover:text-white hover:border-zinc-500'}"
        >{fmt.label}</button>
      {/each}
    </div>

    {#if exportError}
      <p class="text-xs text-red-400 font-mono">{exportError}</p>
    {/if}

    <button
      onclick={doExport}
      disabled={selectedFormats.size === 0}
      class="w-full py-2.5 rounded-lg bg-amber-600 hover:bg-amber-500 text-white text-sm font-semibold transition-colors cursor-pointer disabled:opacity-40"
    >
      Exportera
    </button>

  </div>
</div>
