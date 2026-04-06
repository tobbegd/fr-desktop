<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Settings from "$lib/Settings.svelte";
  import { loadPrefs, savePrefs } from "$lib/store";

  type View = "auth" | "main" | "settings";
  let prevView = $state<View>("main");

  let view = $state<View>("auth");
  let serverUrl = $state(import.meta.env.DEV ? "http://localhost:8081" : "https://foretagsdatabasen.se");
  let apiKey = $state("");
  let email = $state("");
  let tier = $state("");
  let statusMsg = $state("");
  let loading = $state(false);

  // Ladda sparade prefs vid start
  loadPrefs().then(p => {
    if (p.serverUrl) serverUrl = p.serverUrl;
    if (p.apiKey) apiKey = p.apiKey;
    if (p.email) email = p.email;
    if (p.tier) tier = p.tier;
    if (p.apiKey) view = "main";
  });

  function log(msg: string) {
    console.log(`[FDB] ${msg}`);
  }

  async function activate() {
    if (!apiKey || !serverUrl) { statusMsg = "Fyll i alla fält"; return; }
    loading = true;
    statusMsg = "";
    log(`Verifierar mot ${serverUrl}...`);
    try {
      const result = await invoke<{ email: string; tier: string }>("verify_license", { serverUrl, apiKey });
      email = result.email;
      tier = result.tier;
      log(`Verifierad: ${email} (${tier})`);
      await savePrefs({ serverUrl, apiKey, email, tier });
      view = "main";
    } catch (e) {
      const msg = String(e);
      log(`Verifiering misslyckades: ${msg}`);
      if (msg.includes("402") || msg.toLowerCase().includes("prenumeration")) {
        statusMsg = "Din prenumeration har gått ut. Förnya på foretagsdatabasen.se.";
      } else if (msg.includes("401") || msg.toLowerCase().includes("ogiltig")) {
        statusMsg = "Ogiltig API-nyckel.";
      } else {
        statusMsg = msg;
      }
    } finally {
      loading = false;
    }
  }
</script>

{#if view === "auth"}
  <div class="flex items-center justify-center h-screen bg-zinc-950">
    <div class="w-full max-w-md bg-zinc-900 border border-zinc-800 rounded-xl p-8 shadow-2xl">
      <div class="flex items-center justify-between mb-1">
        <h1 class="text-xl font-semibold text-white">Företagsdatabasen – Desktop</h1>
        {#if prevView !== "auth"}
          <button
            class="text-zinc-500 hover:text-white transition-colors cursor-pointer text-lg leading-none"
            onclick={() => view = prevView}
          >✕</button>
        {/if}
      </div>
      <p class="text-zinc-400 text-sm mb-6">Ange din API-nyckel för att aktivera appen.</p>

      <div class="flex flex-col gap-4">
        <div>
          <label class="text-xs text-zinc-400 mb-1 block">Server</label>
          <input
            type="text"
            bind:value={serverUrl}
            class="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-white placeholder-zinc-500 focus:outline-none focus:border-zinc-500"
          />
        </div>
        <div>
          <label class="text-xs text-zinc-400 mb-1 block">API-nyckel</label>
          <input
            type="password"
            bind:value={apiKey}
            placeholder="din nyckel från kontosidan"
            class="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-white placeholder-zinc-500 focus:outline-none focus:border-zinc-500"
          />
        </div>
        {#if statusMsg}
          <p class="text-red-400 text-sm">{statusMsg}</p>
        {/if}
        <button
          class="w-full bg-white text-zinc-900 font-medium rounded-lg py-2 text-sm hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-50"
          onclick={activate}
          disabled={loading}
        >
          {loading ? "Verifierar..." : "Aktivera"}
        </button>
      </div>
    </div>
  </div>

{:else if view === "settings"}
  <Settings
    {serverUrl}
    {apiKey}
    {email}
    {tier}
    onChangeKey={() => { prevView = "settings"; view = "auth"; statusMsg = ""; }}
    onClose={() => view = "main"}
  />

{:else}
  <div class="flex flex-col h-screen bg-zinc-950 text-white">
    <!-- Topmeny -->
    <header class="h-10 flex items-center justify-between px-4 border-b border-zinc-800 shrink-0">
      <span class="text-sm font-medium text-zinc-300">Företagsdatabasen</span>
        <span class="text-xs text-zinc-500">{email}</span>
      <nav class="flex items-center gap-1">
        <button
          class="px-3 py-1 text-sm text-zinc-400 hover:text-white hover:bg-zinc-800 rounded-md transition-colors cursor-pointer"
          onclick={() => view = "settings"}
        >
          Settings
        </button>
      </nav>
    </header>

    <!-- Huvud-UI -->
    <main class="flex-1 flex flex-col p-4 gap-3">
      <p class="text-zinc-500 text-sm">Sök-UI kommer här</p>
    </main>
  </div>
{/if}
