<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  type Props = { dbPath: string };
  let { dbPath }: Props = $props();

  type Stats = {
    total: number;
    med_email: number;
    med_webbadress: number;
    med_arsredovisning: number;
    med_koordinater: number;
    generated_at: string;
  };

  let stats = $state<Stats | null>(null);

  $effect(() => {
    if (!dbPath) return;
    invoke<Stats>("get_db_stats", { dbPath }).then(s => stats = s).catch(() => {});
  });

  function fmt(n: number): string {
    return n.toLocaleString("sv-SE");
  }

  function fmtDate(s: string): string {
    if (!s) return "";
    const d = new Date(s);
    return d.toLocaleDateString("sv-SE", { year: "numeric", month: "long", day: "numeric" });
  }

  function pct(n: number, total: number): number {
    if (!total) return 0;
    return Math.round(n * 100 / total);
  }

  const bars = $derived(stats ? [
    { label: "E-postadress",    value: stats.med_email,          color: "#3b82f6" },
    { label: "Webbadress",      value: stats.med_webbadress,     color: "#10b981" },
    { label: "Årsredovisning",  value: stats.med_arsredovisning, color: "#8b5cf6" },
    { label: "Koordinater",     value: stats.med_koordinater,    color: "#f59e0b" },
  ] : []);
</script>

<div class="flex-1 flex items-center justify-center p-8 select-none">
  {#if !stats}
    <p class="text-zinc-600 text-sm">Laddar...</p>
  {:else}
    <div class="w-full max-w-lg">
      <!-- Stor siffra -->
      <div class="text-center mb-8">
        <div class="text-5xl font-bold text-white tracking-tight">{fmt(stats.total)}</div>
        <div class="text-zinc-500 text-sm mt-1">aktiva bolag</div>
        {#if stats.generated_at}
          <div class="text-zinc-700 text-xs mt-1">{fmtDate(stats.generated_at)}</div>
        {/if}
      </div>

      <!-- Staplar -->
      <div class="flex flex-col gap-3">
        {#each bars as bar}
          {@const p = pct(bar.value, stats!.total)}
          <div>
            <div class="flex justify-between text-xs mb-1">
              <span class="text-zinc-400">{bar.label}</span>
              <span class="text-zinc-400">{fmt(bar.value)}</span>
            </div>
            <div class="h-2 bg-zinc-800 rounded-full overflow-hidden">
              <div
                class="h-full rounded-full transition-all duration-700"
                style="width: {p}%; background: {bar.color};"
              ></div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
