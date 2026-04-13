<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { fade } from "svelte/transition";
  import Settings from "$lib/Settings.svelte";
  import SearchArea from "$lib/SearchArea.svelte";
  import { loadPrefs, savePrefs } from "$lib/store";
  import { showStatus, clearStatus, status } from "$lib/status.svelte";

  type View = "auth" | "main" | "settings";
  let prevView = $state<View>("auth");

  let view = $state<View>("auth");
  let serverUrl = $state(import.meta.env.DEV ? "http://localhost:8081" : "https://foretagsdatabasen.se");
  let apiKey = $state("");
  let email = $state("");
  let tier = $state("");
  let statusMsg = $state("");
  let loading = $state(false);

  // Sparade DB-prefs
  let dbEtag = $state("");
  let dbSha256 = $state("");
  let dbPath = $state("");
  let dbExportDate = $state("");

  // Offline-inloggningar (nödläge)
  const OFFLINE_MAX = 2;
  let offlineLogins = $state(0);
  let isOnline = $state<boolean | null>(null); // null = kontroll pågår

  // Uppdateringsstatus
  let updateAvailable = $state(false);
  let updateFile = $state<{ name: string; size: number; sha256: string } | null>(null);
  let updateEtag = $state("");
  let hasCheckedUpdate = $state(false);

  // Nedladdningsstatus
  let downloading = $state(false);
  let downloadProgress = $state(0);

  // Ladda sparade prefs vid start
  loadPrefs().then(async p => {
    if (p.serverUrl) serverUrl = p.serverUrl;
    if (p.apiKey) apiKey = p.apiKey;
    if (p.email) email = p.email;
    if (p.tier) tier = p.tier;
    if (p.dbEtag) dbEtag = p.dbEtag;
    if (p.dbSha256) dbSha256 = p.dbSha256;
    if (p.dbPath) dbPath = p.dbPath;
    if (p.dbExportDate) dbExportDate = p.dbExportDate;
    offlineLogins = p.offlineLogins ?? 0;

    if (p.apiKey) {
      view = "main";
      const inOfflineMode = (p.offlineLogins ?? 0) > 0;

      if (inOfflineMode) {
        // Offline-läge: kontrollera varje start tills vi får kontakt
        showStatus("autentiserar ...", "info", 0);
        verifyInBackground(p.serverUrl || serverUrl, p.apiKey);
      } else {
        // Normalt läge: kontrollera var 3:e start
        isOnline = true;
        const newCount = (p.launchCount ?? 0) + 1;
        await savePrefs({ launchCount: newCount });
        if (newCount % 3 === 0) {
          showStatus("autentiserar ...", "info", 0);
          verifyInBackground(p.serverUrl || serverUrl, p.apiKey);
        } else {
          const next = 3 - (newCount % 3);
          showStatus(`autentisering om (${next})`, "info", 4000);
        }
      }
    }
  });

  async function verifyInBackground(url: string, key: string) {
    try {
      await invoke<{ email: string; tier: string }>("verify_license", { serverUrl: url, apiKey: key });
      // Lyckades — tillbaka till normalt läge
      isOnline = true;
      clearStatus();
      if (offlineLogins > 0) {
        offlineLogins = 0;
        await savePrefs({ offlineLogins: 0 });
      }
    } catch (e) {
      const msg = String(e);
      const isAuthError = msg.includes("401") || msg.includes("402")
        || msg.toLowerCase().includes("prenumeration")
        || msg.toLowerCase().includes("ogiltig");
      if (!isAuthError) {
        isOnline = false;
        offlineLogins = Math.min(offlineLogins + 1, OFFLINE_MAX);
        await savePrefs({ offlineLogins });
        if (offlineLogins >= OFFLINE_MAX) {
          showStatus(`Servern inte nåbar. Anslut till internet för att fortsätta använda appen.`, "error", 0);
        } else {
          showStatus(`Servern inte nåbar. Offline-läge (${offlineLogins}/${OFFLINE_MAX}).`, "info", 0);
        }
      }
    }
  }

  // Kör manifest-check en gång per session när vi är i main-vyn
  $effect(() => {
    if (view === "main" && !hasCheckedUpdate && tier) {
      hasCheckedUpdate = true;
      checkForUpdate();
    }
  });

  function log(msg: string) {
    console.log(`[FDB] ${msg}`);
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  async function checkForUpdate() {
    log(`Kontrollerar manifest för tier=${tier}...`);
    try {
      const result = await invoke<{
        needs_update: boolean;
        file: { name: string; size: number; sha256: string } | null;
        etag: string;
      }>("check_manifest", {
        serverUrl,
        tier,
        currentEtag: dbEtag,
        currentSha256: dbSha256,
      });
      if (result.needs_update && result.file) {
        log(`Uppdatering tillgänglig: ${result.file.name} (${formatBytes(result.file.size)})`);
        updateAvailable = true;
        updateFile = result.file;
        updateEtag = result.etag;
      } else {
        log("Databasen är aktuell.");
      }
    } catch (e) {
      log(`Manifest-kontroll misslyckades: ${e}`);
    }
  }

  async function doDownload() {
    if (!updateFile) return;
    downloading = true;
    downloadProgress = 0;
    log(`Laddar ner ${updateFile.name}...`);

    const unlisten = await listen<{ downloaded: number; total: number }>(
      "download-progress",
      (event) => {
        const { downloaded, total } = event.payload;
        if (total > 0) {
          downloadProgress = Math.round((downloaded / total) * 100);
        }
      }
    );

    try {
      const downloadedPath = await invoke<string>("download_db", {
        serverUrl,
        apiKey,
        expectedSha256: updateFile.sha256,
        fileName: updateFile.name,
      });
      log(`Databas sparad: ${dbPath}`);
      const exportDate = updateFile.name.replace(/\.(sqlite|db|zip)$/i, "").split("_")[1] ?? "";
      await savePrefs({ dbEtag: updateEtag, dbSha256: updateFile.sha256, dbPath: downloadedPath, dbExportDate: exportDate });
      dbEtag = updateEtag;
      dbSha256 = updateFile.sha256;
      dbPath = downloadedPath;
      dbExportDate = exportDate;
      updateAvailable = false;
      updateFile = null;
      showStatus("Databasen är uppdaterad.", "success");
    } catch (e) {
      log(`Nedladdning misslyckades: ${e}`);
      showStatus(String(e), "error");
    } finally {
      unlisten();
      downloading = false;
    }
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
      offlineLogins = 0;
      await savePrefs({ serverUrl, apiKey, email, tier, offlineLogins: 0 });
      hasCheckedUpdate = false;
      view = "main";
    } catch (e) {
      const msg = String(e);
      log(`Verifiering misslyckades: ${msg}`);
      if (msg.includes("402") || msg.toLowerCase().includes("prenumeration")) {
        statusMsg = "Din prenumeration har gått ut. Förnya på foretagsdatabasen.se.";
      } else if (msg.includes("401") || msg.toLowerCase().includes("ogiltig")) {
        statusMsg = "Ogiltig API-nyckel.";
      } else {
        statusMsg = "Servern är inte nåbar. Kontrollera din anslutning.";
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
    {dbExportDate}
    onChangeKey={() => { prevView = "settings"; view = "auth"; statusMsg = ""; }}
    onClose={() => view = "main"}
  />

{:else}
  <div class="flex flex-col h-screen bg-zinc-950 text-white">
    <!-- Topmeny -->
    <header class="h-10 flex items-center justify-between px-4 border-b border-zinc-800 shrink-0">
      <span class="text-sm font-medium text-zinc-300">Företagsdatabasen</span>
      <span class="text-xs text-zinc-500 flex items-center gap-2">
        {email}
        {#if isOnline === true}
          <span class="text-green-500" title="Ansluten">●</span>
        {:else if isOnline === false}
          <span class="text-yellow-500" title="Offline ({offlineLogins}/{OFFLINE_MAX})">● offline ({offlineLogins}/{OFFLINE_MAX})</span>
        {/if}
      </span>
      <nav class="flex items-center gap-1">
        <button
          class="px-3 py-1 text-sm text-zinc-400 hover:text-white hover:bg-zinc-800 rounded-md transition-colors cursor-pointer"
          onclick={() => view = "settings"}
        >
          Settings
        </button>
      </nav>
    </header>

    <!-- Uppdateringsbanner -->
    {#if updateAvailable && !downloading}
      <div class="flex items-center justify-between px-4 py-2 bg-zinc-900 border-b border-zinc-800">
        <span class="text-sm text-zinc-300">
          Ny version av databasen tillgänglig
          {#if updateFile} — {formatBytes(updateFile.size)}{/if}
        </span>
        <div class="flex items-center gap-2">
          <button
            class="px-3 py-1 text-xs bg-white text-zinc-900 font-medium rounded-md hover:bg-zinc-200 transition-colors cursor-pointer"
            onclick={doDownload}
          >
            Ladda ner
          </button>
          <button
            class="px-3 py-1 text-xs text-zinc-500 hover:text-white transition-colors cursor-pointer"
            onclick={() => { updateAvailable = false; }}
          >
            Senare
          </button>
        </div>
      </div>
    {:else if downloading}
      <div class="px-4 py-2.5 bg-zinc-900 border-b border-zinc-800">
        <div class="flex justify-between text-xs text-zinc-400 mb-1.5">
          <span>Laddar ner databas...</span>
          <span>{downloadProgress}%</span>
        </div>
        <div class="h-1 bg-zinc-800 rounded-full overflow-hidden">
          <div
            class="h-full bg-white rounded-full transition-all duration-200"
            style="width: {downloadProgress}%"
          ></div>
        </div>
      </div>
    {/if}

    <!-- Statusrad -->
    {#if status.message}
      <div
        transition:fade={{ duration: 200 }}
        class="px-4 py-2 border-b border-zinc-800 text-sm
          {status.type === 'success' ? 'text-green-400' : ''}
          {status.type === 'error' ? 'text-red-400' : ''}
          {status.type === 'info' ? 'text-zinc-400' : ''}"
      >
        {status.message}
      </div>
    {/if}

    <SearchArea {dbPath} />
  </div>
{/if}
