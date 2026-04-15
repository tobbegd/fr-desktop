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
  let selectedFormat = $state<Format>("csv");

  const formats: { id: Format; label: string }[] = [
    { id: "csv",  label: "CSV" },
    { id: "xlsx", label: "Excel" },
    { id: "json", label: "JSON" },
  ];

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
    return lines.join("\n");
  }

  function buildXlsx(): Uint8Array {
    const data = [columns, ...rows.map(r => r.map(c => c === null || c === undefined ? "" : c))];
    const ws = XLSX.utils.aoa_to_sheet(data);
    const wb = XLSX.utils.book_new();
    XLSX.utils.book_append_sheet(wb, ws, "Export");
    return XLSX.write(wb, { type: "array", bookType: "xlsx" });
  }

  function buildJson(): string {
    const data = rows.map(row =>
      Object.fromEntries(columns.map((col, i) => [col, row[i] ?? null]))
    );
    return JSON.stringify(data, null, 2);
  }

  async function doExport() {
    if (selectedFormat === "csv") {
      const content = buildCsv();
      await invoke("save_file", { filename: "export.csv", content, extension: "csv" }).catch(() => {});
    } else if (selectedFormat === "xlsx") {
      const bytes = buildXlsx();
      const content = Array.from(bytes).map(b => String.fromCharCode(b)).join("");
      await invoke("save_file_binary", { filename: "export.xlsx", bytes: Array.from(bytes) }).catch(() => {});
    } else if (selectedFormat === "json") {
      const content = buildJson();
      await invoke("save_file", { filename: "export.json", content, extension: "json" }).catch(() => {});
    }
    onclose();
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
          onclick={() => selectedFormat = fmt.id}
          class="flex-1 py-2 rounded-lg text-sm font-medium border transition-colors cursor-pointer
            {selectedFormat === fmt.id
              ? 'bg-amber-600 border-amber-500 text-white'
              : 'bg-zinc-800 border-zinc-700 text-zinc-400 hover:text-white hover:border-zinc-500'}"
        >{fmt.label}</button>
      {/each}
    </div>

    <button
      onclick={doExport}
      class="w-full py-2.5 rounded-lg bg-amber-600 hover:bg-amber-500 text-white text-sm font-semibold transition-colors cursor-pointer"
    >
      Exportera som {formats.find(f => f.id === selectedFormat)?.label}
    </button>

  </div>
</div>
