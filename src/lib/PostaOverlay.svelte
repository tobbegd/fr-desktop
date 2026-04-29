<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { loadPrefs } from "$lib/store";
  import { onMount, onDestroy } from "svelte";

  type Utskick = { id: number; sack_id: number; sack_namn: string; template_id: number; template_namn: string; skapad: string; status: string; fordrojning_sek: number };
  type UtskickInfo = { total: number; reklamsparrade: number; skickade: number; fordrojning_sek: number };
  type Progress = { utskick_id: number; sent: number; total: number; current_orgnamn: string };

  type Props = { utskick: Utskick; onClose: () => void; onDone: () => void };
  let { utskick, onClose, onDone }: Props = $props();

  let info = $state<UtskickInfo | null>(null);
  let hoppaOver = $state(true);
  let skickar = $state(false);
  let progress = $state<Progress | null>(null);
  let fel = $state("");
  let klart = $state(false);

  let totalAttSkicka = $derived(info ? (hoppaOver ? info.total - info.reklamsparrade : info.total) : 0);
  let kvarAtSkicka = $derived(info ? Math.max(0, totalAttSkicka - info.skickade) : 0);

  let unlisten: (() => void) | null = null;

  onMount(async () => {
    info = await invoke<UtskickInfo>("get_utskick_info", { utskickId: utskick.id });
    const ul = await listen<Progress>("utskick-progress", (event) => {
      if (event.payload.utskick_id === utskick.id) {
        progress = event.payload;
      }
    });
    unlisten = ul;
  });

  onDestroy(() => { unlisten?.(); });

  async function starta() {
    fel = "";
    klart = false;
    skickar = true;
    progress = null;
    try {
      const p = await loadPrefs();
      if (!p.smtpHost) throw new Error("SMTP ej konfigurerat. Gå till Inställningar.");
      await invoke("post_utskick", {
        utskickId: utskick.id,
        hoppaOverReklamsparr: hoppaOver,
        host: p.smtpHost,
        port: p.smtpPort ?? 587,
        encryption: p.smtpEncryption ?? "starttls",
        username: p.smtpUsername ?? "",
        password: p.smtpPassword ?? "",
        fromName: p.smtpFromName ?? "",
        fromEmail: p.smtpFromEmail ?? "",
      });
      klart = true;
      info = await invoke<UtskickInfo>("get_utskick_info", { utskickId: utskick.id });
      onDone();
    } catch (e) {
      fel = String(e);
    } finally {
      skickar = false;
    }
  }
</script>

<div
  class="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
  role="dialog"
  aria-modal="true"
  onclick={onClose}
>
  <div
    class="bg-zinc-900 border border-zinc-700 rounded-lg w-[480px] max-w-full mx-4 shadow-2xl"
    onclick={(e) => e.stopPropagation()}
    role="presentation"
  >
    <!-- Header -->
    <div class="px-6 py-4 border-b border-zinc-800 flex items-start justify-between">
      <div>
        <p class="text-sm font-medium text-zinc-100">Posta utskick</p>
        <p class="text-xs text-zinc-500 mt-0.5">{utskick.template_namn} → {utskick.sack_namn}</p>
      </div>
      <button onclick={onClose} class="text-zinc-500 hover:text-zinc-200 text-sm cursor-pointer ml-4 shrink-0">✕</button>
    </div>

    <div class="px-6 py-5 space-y-5">
      <!-- Riskvarning -->
      <div class="bg-amber-950/50 border border-amber-800/60 rounded-md px-4 py-3 text-xs text-amber-300 leading-relaxed">
        <strong>Risk:</strong> Att skicka stora mailvolymer kan leda till att din avsändardomän eller ditt SMTP-konto spärras av spamfilter. Fördröjningen ({utskick.fordrojning_sek}s) minskar risken men eliminerar den inte.
      </div>

      <!-- Statistik -->
      {#if info}
        <div class="space-y-2 text-xs">
          <div class="flex justify-between">
            <span class="text-zinc-400">Bolag i säcken (med e-post)</span>
            <span class="text-zinc-200">{info.total}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-zinc-400">Har reklamsparr</span>
            <span class="{info.reklamsparrade > 0 ? 'text-amber-400' : 'text-zinc-500'}">{info.reklamsparrade}</span>
          </div>
          {#if info.skickade > 0}
            <div class="flex justify-between">
              <span class="text-zinc-400">Redan skickade</span>
              <span class="text-green-400">{info.skickade}</span>
            </div>
          {/if}
          <div class="border-t border-zinc-800 pt-2 flex justify-between font-medium text-sm">
            <span class="text-zinc-300">Skickas nu</span>
            <span class="text-white">{kvarAtSkicka}</span>
          </div>
        </div>

        <!-- Reklamsparr-checkbox -->
        {#if info.reklamsparrade > 0}
          <label class="flex items-center gap-3 cursor-pointer group">
            <input
              type="checkbox"
              bind:checked={hoppaOver}
              disabled={skickar}
              class="w-4 h-4 rounded border-zinc-600 bg-zinc-800 accent-white cursor-pointer disabled:cursor-not-allowed"
            />
            <span class="text-xs text-zinc-300 group-hover:text-white transition-colors select-none">
              Hoppa över bolag med reklamsparr ({info.reklamsparrade} st)
            </span>
          </label>
        {/if}
      {:else}
        <div class="text-xs text-zinc-600 text-center py-4">Laddar...</div>
      {/if}

      <!-- Progress-bar -->
      {#if skickar && progress}
        <div class="space-y-2">
          <div class="flex justify-between text-xs text-zinc-400">
            <span class="truncate mr-2">Skickar till {progress.current_orgnamn}...</span>
            <span class="shrink-0">{progress.sent}/{progress.total}</span>
          </div>
          <div class="w-full bg-zinc-800 rounded-full h-1.5">
            <div
              class="bg-white h-1.5 rounded-full transition-all duration-300"
              style="width: {(progress.sent / progress.total) * 100}%"
            ></div>
          </div>
        </div>
      {/if}

      <!-- Klart-meddelande -->
      {#if klart}
        <p class="text-xs text-green-400 bg-green-950/40 border border-green-800/40 rounded px-3 py-2">
          Klart! {info?.skickade ?? ""} mail skickade.
        </p>
      {/if}

      <!-- Felmeddelande -->
      {#if fel}
        <p class="text-xs text-red-400 bg-red-950/40 border border-red-800/40 rounded px-3 py-2">{fel}</p>
      {/if}
    </div>

    <!-- Footer -->
    <div class="px-6 pb-5 flex gap-2 justify-end">
      {#if !skickar}
        <button onclick={onClose} class="px-4 py-1.5 text-xs text-zinc-400 hover:text-white transition-colors cursor-pointer">
          {klart ? "Stäng" : "Avbryt"}
        </button>
        {#if info && kvarAtSkicka > 0}
          <button
            onclick={starta}
            class="px-4 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded hover:bg-zinc-200 transition-colors cursor-pointer"
          >
            {info.skickade > 0 ? `Fortsätt (${info.skickade} → ${totalAttSkicka})` : "Starta utskick"}
          </button>
        {/if}
      {:else}
        <button disabled class="px-4 py-1.5 text-xs bg-zinc-700 text-zinc-400 rounded cursor-not-allowed">
          Skickar...
        </button>
      {/if}
    </div>
  </div>
</div>
