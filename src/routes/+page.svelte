<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { check as checkUpdate, type Update } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { fade } from "svelte/transition";
  import Settings from "$lib/Settings.svelte";
  import SearchArea from "$lib/SearchArea.svelte";
  import MenuBar from "$lib/MenuBar.svelte";
  import type { MenuDef, MenuItem } from "$lib/MenuBar.svelte";
  import { loadPrefs, savePrefs } from "$lib/store";
  import DebugConsole from "$lib/DebugConsole.svelte";
  import MailPage from "$lib/MailPage.svelte";
  import { debug } from "$lib/debug.svelte";
  import { appearance } from "$lib/appearance.svelte";
  import { showStatus, clearStatus, status } from "$lib/status.svelte";
  import MessagesPanel from "$lib/MessagesPanel.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  type View = "auth" | "main" | "settings" | "mail" | "demo-expired";
  let prevView = $state<View>("auth");

  let view = $state<View>("auth");
  let settingsInitialSection = $state("general");
  let serverUrl = $state(import.meta.env.VITE_SERVER_URL ?? (import.meta.env.DEV ? "http://localhost:8081" : "https://foretagsdatabasen.se"));
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

  // Nedladdningsstatus (databas)
  let downloading = $state(false);
  let downloadProgress = $state(0);

  // App-uppdatering
  let appUpdate = $state<Update | null>(null);
  let appUpdating = $state(false);
  let appUpdateProgress = $state(0);

  // Meddelanden & frågor
  let pendingQuestions = $state<{ id: number; body: string }[]>([]);
  let canMessage = $state(false);
  let showMessagesPanel = $state(false);
  let messagesPollTimer: ReturnType<typeof setInterval> | null = null;

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
    debug.console = p.debugConsole ?? false;
    debug.ai = p.debugAi ?? false;
    appearance.tableFontSize = p.tableFontSize ?? 12;
    appearance.editorFontSize = p.editorFontSize ?? 14;
    appearance.collapseSearch = p.collapseSearch ?? true;

    if (p.apiKey) {
      view = "main";
      const inOfflineMode = (p.offlineLogins ?? 0) > 0;

      if (inOfflineMode) {
        // Offline-läge: kontrollera varje start tills vi får kontakt
        showStatus("autentiserar ...", "info", 0);
        verifyInBackground(p.serverUrl || serverUrl, p.apiKey);
      } else {
        // Demo-tier verifierar alltid — demotiden kan löpa ut när som helst
        isOnline = true;
        if (p.tier === "demo") {
          showStatus("autentiserar ...", "info", 0);
          verifyInBackground(p.serverUrl || serverUrl, p.apiKey);
        } else {
          // Normalt läge: kontrollera var 3:e start
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
    }
  });

  async function verifyInBackground(url: string, key: string) {
    try {
      const result = await invoke<{ email: string; tier: string }>("verify_license", { serverUrl: url, apiKey: key });
      // Lyckades — tillbaka till normalt läge
      isOnline = true;
      clearStatus();
      const updates: Record<string, unknown> = {};
      if (result.tier !== tier) { tier = result.tier; updates.tier = result.tier; }
      if (offlineLogins > 0) { offlineLogins = 0; updates.offlineLogins = 0; }
      if (Object.keys(updates).length > 0) await savePrefs(updates);
    } catch (e) {
      const msg = String(e);
      if (msg.includes("402") || msg.toLowerCase().includes("demotiden")) {
        view = "demo-expired";
        return;
      }
      const isAuthError = msg.includes("401")
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
      checkDbFileExists().then(() => {
        checkForUpdate();
        checkForAppUpdate();
      });
    }
  });

  async function checkDbFileExists() {
    if (!dbPath) return;
    const exists = await invoke<boolean>("file_exists", { path: dbPath });
    if (!exists) {
      log("DB-fil saknas lokalt — återställer etag/sha256 för att tvinga ny nedladdning");
      dbEtag = "";
      dbSha256 = "";
      await savePrefs({ dbEtag: "", dbSha256: "" });
    }
  }


  async function pollMessages() {
    if (!apiKey || !isOnline) return;
    try {
      const result = await invoke<{ can_message: boolean; questions: { id: number; body: string }[] }>(
        "fetch_questions", { serverUrl, apiKey }
      );
      pendingQuestions = result.questions;
      canMessage = result.can_message;
    } catch {
      // ignorera nätverksfel tyst
    }
  }

  $effect(() => {
    if (view === "main" && isOnline === true && apiKey) {
      pollMessages();
      if (messagesPollTimer) clearInterval(messagesPollTimer);
      messagesPollTimer = setInterval(pollMessages, 5 * 60 * 1000);
      return () => {
        if (messagesPollTimer) clearInterval(messagesPollTimer);
      };
    }
  });

  $effect(() => {
    document.documentElement.style.setProperty("--table-font-size", `${appearance.tableFontSize}px`);
    document.documentElement.style.setProperty("--editor-font-size", `${appearance.editorFontSize}px`);
  });

  function log(msg: string) {
    console.log(`[FDB] ${msg}`);
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  async function checkForUpdate() {
    const manifestTier = tier === "demo" || tier === "desktop" ? "pro" : tier;
    log(`Kontrollerar manifest för tier=${tier} (manifest: ${manifestTier})...`);
    try {
      const result = await invoke<{
        needs_update: boolean;
        file: { name: string; size: number; sha256: string } | null;
        etag: string;
      }>("check_manifest", {
        serverUrl,
        tier: manifestTier,
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

  async function checkForAppUpdate() {
    try {
      const update = await checkUpdate();
      if (update) appUpdate = update;
    } catch {
      // ignorera tyst — uppdateringskollen är best-effort
    }
  }

  async function doAppUpdate() {
    if (!appUpdate) return;
    appUpdating = true;
    appUpdateProgress = 0;
    try {
      let downloaded = 0;
      let total = 0;
      await appUpdate.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          if (total > 0) appUpdateProgress = Math.round((downloaded / total) * 100);
        }
      });
      await relaunch();
    } catch (e) {
      showStatus(String(e), "error");
      appUpdating = false;
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

  let actionMenuItems = $state<MenuItem[]>([]);
  let mailMenuItems = $state<MenuItem[]>([]);
  let kartaMenuItems = $state<MenuItem[]>([]);
  let showSqlEditor = $state(false);

  const appMenus: MenuDef[] = $derived([
    {
      label: "Företagsdatabasen",
      items: [
        { label: "Inställningar", action: () => { prevView = view; settingsInitialSection = "general"; view = "settings"; } },
        { separator: true },
        { label: "Avsluta", shortcut: "Ctrl+Q", action: () => invoke("quit") },
      ],
    },
    {
      label: "Åtgärder",
      items: actionMenuItems,
    },
    {
      label: "Karta",
      items: kartaMenuItems,
    },
    {
      label: "Utskick",
      items: [
        { label: "Öppna mailutskick", action: () => { prevView = view; view = "mail"; } },
        { separator: true },
        ...mailMenuItems,
      ],
    },
    {
      label: "Fönster",
      items: [
        { label: showSqlEditor ? "Dölj SQL-editor" : "Visa SQL-editor", action: () => { showSqlEditor = !showSqlEditor; } },
      ],
    },
  ]);

  function onKeydown(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === "q") { e.preventDefault(); invoke("quit"); }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if debug.console}<DebugConsole />{/if}

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

{:else if view === "demo-expired"}
  <div class="flex items-center justify-center h-screen bg-zinc-950">
    <div class="w-full max-w-md bg-zinc-900 border border-zinc-800 rounded-xl p-8 shadow-2xl text-center">
      <div class="text-4xl mb-4">⏱</div>
      <h1 class="text-xl font-semibold text-white mb-2">Din demotid har löpt ut</h1>
      <p class="text-zinc-400 text-sm mb-6">Fortsätt använda Företagsdatabasen genom att skaffa en prenumeration.</p>
      <button
        class="w-full bg-white text-zinc-900 font-medium rounded-lg py-2 text-sm hover:bg-zinc-200 transition-colors cursor-pointer"
        onclick={() => openUrl(`${serverUrl}/set-password`)}
      >
        Fortsätt efter demotiden
      </button>
    </div>
  </div>

{:else if view === "mail"}
  <MailPage onClose={() => { view = prevView; }} />

{:else}
  <div class="flex flex-col h-screen bg-zinc-950 text-white">
    <!-- Topmeny -->
    <header class="relative h-10 flex items-center justify-between px-2 border-b border-zinc-800 shrink-0">
      <MenuBar menus={appMenus} />


      <span class="text-xs text-zinc-500 flex items-center gap-2 px-2">
        {#if isOnline === true && apiKey}
          <button
            onclick={() => showMessagesPanel = !showMessagesPanel}
            class="relative text-zinc-400 hover:text-zinc-200 transition-colors cursor-pointer"
            title="Meddelanden"
            aria-label="Meddelanden"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
            </svg>
            {#if pendingQuestions.length > 0}
              <span class="absolute -top-1 -right-1 w-2 h-2 bg-blue-500 rounded-full"></span>
            {/if}
          </button>
        {/if}
        {email}
        {#if isOnline === true}
          <span class="text-green-500" title="Ansluten">●</span>
        {:else if isOnline === false}
          <span class="text-yellow-500" title="Offline ({offlineLogins}/{OFFLINE_MAX})">● offline ({offlineLogins}/{OFFLINE_MAX})</span>
        {/if}
      </span>
    </header>
    <!-- App-uppdateringsbanner -->
    {#if appUpdate && !appUpdating}
      <div class="flex items-center justify-between px-4 py-2 bg-zinc-900 border-b border-zinc-800">
        <span class="text-sm text-zinc-300">
          Ny version av appen tillgänglig — {appUpdate.version}
          {#if appUpdate.body}<span class="text-zinc-500 text-xs ml-2">{appUpdate.body}</span>{/if}
        </span>
        <button
          class="px-3 py-1 text-xs bg-white text-zinc-900 font-medium rounded-md hover:bg-zinc-200 transition-colors cursor-pointer"
          onclick={doAppUpdate}
        >Installera och starta om</button>
      </div>
    {:else if appUpdating}
      <div class="px-4 py-2.5 bg-zinc-900 border-b border-zinc-800">
        <div class="flex justify-between text-xs text-zinc-400 mb-1.5">
          <span>Installerar appuppdatering...</span>
          <span>{appUpdateProgress}%</span>
        </div>
        <div class="h-1 bg-zinc-800 rounded-full overflow-hidden">
          <div class="h-full bg-white rounded-full transition-all duration-200" style="width: {appUpdateProgress}%"></div>
        </div>
      </div>
    {/if}

    <!-- Databasuppdateringsbanner -->
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

    <SearchArea
      {dbPath}
      onOpenAiSettings={() => { prevView = view; settingsInitialSection = "ai"; view = "settings"; }}
      bind:showSqlEditor
      bind:actionMenuItems
      bind:mailMenuItems
      bind:kartaMenuItems
      collapseSearch={appearance.collapseSearch}
    />

    {#if view === "settings"}
      <div class="fixed inset-0 z-50 bg-zinc-950">
        <Settings
          {serverUrl}
          {apiKey}
          {email}
          {tier}
          {dbExportDate}
          {dbPath}
          initialSection={settingsInitialSection}
          onChangeKey={() => { prevView = "settings"; view = "auth"; statusMsg = ""; }}
          onLogout={async () => {
            await savePrefs({ apiKey: "", email: "", tier: "", offlineLogins: 0, dbEtag: "", dbSha256: "" });
            apiKey = ""; email = ""; tier = ""; offlineLogins = 0; dbEtag = ""; dbSha256 = "";
            statusMsg = "";
            view = "auth";
          }}
          onClose={async () => {
            view = "main";
            settingsInitialSection = "general";
            const p = await loadPrefs();
          }}
        />
      </div>
    {/if}

    {#if showMessagesPanel}
      <MessagesPanel
        {serverUrl}
        {apiKey}
        {canMessage}
        questions={pendingQuestions}
        onClose={() => showMessagesPanel = false}
        onRefresh={pollMessages}
      />
    {/if}
  </div>
{/if}
