<script lang="ts">
  import { invoke as tauri } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { Chart, LineController, LineElement, PointElement, LinearScale, CategoryScale, Tooltip, Legend, Filler } from "chart.js";

  Chart.register(LineController, LineElement, PointElement, LinearScale, CategoryScale, Tooltip, Legend, Filler);

  type Props = { dbPath: string; orgnr: string; orgnamn: string; onclose: () => void };
  let { dbPath, orgnr, orgnamn, onclose }: Props = $props();

  type ArsRow = {
    ar: string;
    nettoomsattning: number | null;
    arets_resultat: number | null;
    eget_kapital: number | null;
    soliditet: number | null;
    vinstmarginal: number | null;
    kassalikviditet: number | null;
    medelantal_anstallda: number | null;
  };

  let rows = $state<ArsRow[]>([]);
  let loading = $state(true);
  let error = $state("");

  const CHARTS: { key: keyof ArsRow; label: string; unit: string; color: string }[] = [
    { key: "nettoomsattning",    label: "Nettoomsättning",    unit: "kr",  color: "#60a5fa" },
    { key: "arets_resultat",     label: "Årets resultat",     unit: "kr",  color: "#34d399" },
    { key: "eget_kapital",       label: "Eget kapital",       unit: "kr",  color: "#a78bfa" },
    { key: "soliditet",          label: "Soliditet",          unit: "%",   color: "#fb923c" },
    { key: "vinstmarginal",      label: "Vinstmarginal",      unit: "%",   color: "#f472b6" },
    { key: "kassalikviditet",    label: "Kassalikviditet",    unit: "%",   color: "#facc15" },
    { key: "medelantal_anstallda", label: "Medelantal anst.", unit: "st",  color: "#2dd4bf" },
  ];

  let canvases = $state<(HTMLCanvasElement | null)[]>(Array(CHARTS.length).fill(null));
  let charts: (Chart | null)[] = Array(CHARTS.length).fill(null);

  function fmt(v: number | null, unit: string): string {
    if (v === null) return "–";
    if (unit === "kr") {
      if (Math.abs(v) >= 1_000_000) return (v / 1_000_000).toFixed(1) + " Mkr";
      if (Math.abs(v) >= 1_000) return (v / 1_000).toFixed(0) + " tkr";
      return v.toFixed(0) + " kr";
    }
    if (unit === "%") return v.toFixed(1) + " %";
    return v.toFixed(0) + " " + unit;
  }

  async function load() {
    loading = true;
    error = "";
    try {
      const res = await tauri<{ columns: string[]; rows: unknown[][] }>("query_db", {
        dbPath,
        sql: `SELECT substr(rakenskapsar_slut, 1, 4) as ar,
                     nettoomsattning, arets_resultat, eget_kapital,
                     soliditet, vinstmarginal, kassalikviditet, medelantal_anstallda
              FROM arsredovisningar
              WHERE replace(orgnr, '-', '') = replace('${orgnr}', '-', '')
              ORDER BY rakenskapsar_slut ASC`,
      });
      const cols = res.columns;
      rows = res.rows.map(r => {
        const o: Record<string, unknown> = {};
        cols.forEach((c, i) => o[c] = r[i]);
        return o as unknown as ArsRow;
      });
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function buildCharts() {
    const labels = rows.map(r => r.ar);
    CHARTS.forEach((cfg, idx) => {
      const canvas = canvases[idx];
      if (!canvas) return;
      charts[idx]?.destroy();
      const data = rows.map(r => r[cfg.key] as number | null);
      charts[idx] = new Chart(canvas, {
        type: "line",
        data: {
          labels,
          datasets: [{
            label: cfg.label,
            data,
            borderColor: cfg.color,
            backgroundColor: cfg.color + "22",
            fill: true,
            tension: 0.3,
            pointRadius: 4,
            pointHoverRadius: 6,
          }],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: {
            legend: { display: false },
            tooltip: {
              callbacks: {
                label: ctx => fmt(ctx.parsed.y, cfg.unit),
              },
            },
          },
          scales: {
            x: { ticks: { color: "#a1a1aa" }, grid: { color: "#3f3f46" } },
            y: {
              ticks: { color: "#a1a1aa", callback: v => fmt(Number(v), cfg.unit) },
              grid: { color: "#3f3f46" },
            },
          },
        },
      });
    });
  }

  $effect(() => {
    if (!loading && rows.length > 0 && canvases.some(c => c !== null)) {
      buildCharts();
    }
  });

  onMount(() => { load(); });

  onDestroy(() => { charts.forEach(c => c?.destroy()); });

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<!-- Overlay -->
<div
  class="fixed inset-0 z-40 bg-black/60"
  role="button"
  tabindex="-1"
  onclick={onclose}
  onkeydown={() => {}}
></div>

<!-- Panel -->
<div class="fixed inset-4 z-50 flex flex-col rounded-xl border border-zinc-700 bg-zinc-950 shadow-2xl overflow-hidden">
  <!-- Header -->
  <div class="flex items-center justify-between px-5 py-3 border-b border-zinc-800">
    <div>
      <span class="text-base font-semibold text-zinc-100">{orgnamn}</span>
      <span class="ml-3 text-sm text-zinc-500">{orgnr}</span>
    </div>
    <button
      class="text-zinc-400 hover:text-zinc-100 text-xl leading-none px-1"
      onclick={onclose}
    >✕</button>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-y-auto p-5">
    {#if loading}
      <div class="flex items-center justify-center h-40 text-zinc-500">Laddar...</div>
    {:else if error}
      <div class="text-red-400 text-sm">{error}</div>
    {:else if rows.length === 0}
      <div class="flex items-center justify-center h-40 text-zinc-500">Inga bokslut hittades.</div>
    {:else}
      <!-- Latest values summary -->
      {@const latest = rows[rows.length - 1]}
      <div class="flex flex-wrap gap-3 mb-6">
        {#each CHARTS as cfg}
          {@const v = latest[cfg.key] as number | null}
          <div class="rounded-lg bg-zinc-900 border border-zinc-800 px-4 py-2 min-w-28">
            <div class="text-xs text-zinc-500 mb-0.5">{cfg.label}</div>
            <div class="text-sm font-semibold" style="color: {cfg.color}">{fmt(v, cfg.unit)}</div>
          </div>
        {/each}
      </div>

      <!-- Charts grid -->
      <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-5">
        {#each CHARTS as cfg, idx}
          <div class="rounded-lg bg-zinc-900 border border-zinc-800 p-4">
            <div class="text-xs font-medium text-zinc-400 mb-3">{cfg.label}</div>
            <div class="h-40">
              <canvas bind:this={canvases[idx]}></canvas>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
