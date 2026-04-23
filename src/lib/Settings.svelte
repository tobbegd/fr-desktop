<script lang="ts">
  import AiSetup from "$lib/AiSetup.svelte";
  import SmtpSetup from "$lib/SmtpSetup.svelte";
  import { debug } from "$lib/debug.svelte";
  import { savePrefs } from "$lib/store";

  type Props = {
    serverUrl: string;
    apiKey: string;
    email: string;
    tier: string;
    dbExportDate: string;
    dbPath: string;
    initialSection?: string;
    onChangeKey: () => void;
    onClose: () => void;
  };

  let { serverUrl, apiKey, email, tier, dbExportDate, dbPath, initialSection = "general", onChangeKey, onClose }: Props = $props();

  let activeSection = $state(initialSection);

  const nav = [
    { id: "general", label: "Allmänt" },
    { id: "auth", label: "Autentisering" },
    { id: "ai", label: "AI-assistent" },
    { id: "mail", label: "Mailutskick" },
    { id: "debug", label: "Debug" },
  ];

  function maskedKey(key: string) {
    if (!key) return "—";
    return key.slice(0, 6) + "••••••••••••••••••••";
  }
</script>

<div class="flex h-screen bg-zinc-950 text-white">
  <!-- Sidebar -->
  <aside class="w-56 shrink-0 border-r border-zinc-800 flex flex-col">
    <div class="h-12 flex items-center px-4 border-b border-zinc-800">
      <span class="text-sm font-medium text-zinc-100">Inställningar</span>
    </div>
    <nav class="flex-1 p-2">
      {#each nav as item}
        <button
          class="w-full text-left px-3 py-2 rounded-md text-sm transition-colors cursor-pointer
            {activeSection === item.id
              ? 'bg-zinc-800 text-white'
              : 'text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200'}"
          onclick={() => activeSection = item.id}
        >
          {item.label}
        </button>
      {/each}
    </nav>
  </aside>

  <!-- Content -->
  <div class="flex-1 flex flex-col">
    <div class="h-12 shrink-0 flex items-center justify-between px-6 border-b border-zinc-800">
      <span class="text-sm font-medium text-zinc-100">
        {nav.find(n => n.id === activeSection)?.label}
      </span>
      <button
        class="text-sm text-zinc-400 hover:text-white transition-colors cursor-pointer"
        onclick={onClose}
      >
        Stäng
      </button>
    </div>

    <div class="flex-1 overflow-y-auto p-6">
      {#if activeSection === "general"}
        <div class="max-w-md flex flex-col gap-4">
          <div>
            <p class="text-xs text-zinc-500 mb-1">Exportdatum</p>
            <p class="text-sm text-zinc-200 bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2">
              {dbExportDate || "—"}
            </p>
          </div>
        </div>
      {:else if activeSection === "auth"}
        <div class="max-w-md flex flex-col gap-6">
          <div class="flex flex-col gap-4">
            <div>
              <p class="text-xs text-zinc-500 mb-1">E-post</p>
              <p class="text-sm text-zinc-200 bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2">{email}</p>
            </div>
            <div>
              <p class="text-xs text-zinc-500 mb-1">Plan</p>
              <p class="text-sm text-zinc-200 bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 capitalize">{tier}</p>
            </div>
            <div>
              <p class="text-xs text-zinc-500 mb-1">Server</p>
              <p class="text-sm text-zinc-200 bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2">{serverUrl}</p>
            </div>
            <div>
              <p class="text-xs text-zinc-500 mb-1">API-nyckel</p>
              <p class="text-sm text-zinc-200 bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 font-mono">{maskedKey(apiKey)}</p>
            </div>
          </div>
          <button
            class="w-fit bg-zinc-800 hover:bg-zinc-700 text-white text-sm font-medium rounded-lg px-4 py-2 transition-colors cursor-pointer"
            onclick={onChangeKey}
          >
            Ändra nyckel
          </button>
        </div>
      {:else if activeSection === "ai"}
        <AiSetup {dbPath} />
      {:else if activeSection === "mail"}
        <SmtpSetup />
      {:else if activeSection === "debug"}
        <div class="max-w-md flex flex-col gap-6">
          <label class="flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              checked={debug.console}
              onchange={async (e) => {
                debug.console = (e.target as HTMLInputElement).checked;
                await savePrefs({ debugConsole: debug.console });
              }}
              class="accent-zinc-400 w-4 h-4 cursor-pointer"
            />
            <div>
              <p class="text-sm text-zinc-200">Aktivera debug-konsoll</p>
              <p class="text-xs text-zinc-500">Visar ett flytande loggfönster längst ned i appen.</p>
            </div>
          </label>
          <label class="flex items-center gap-3 {debug.console ? 'cursor-pointer' : 'opacity-40 cursor-not-allowed'}">
            <input
              type="checkbox"
              checked={debug.ai}
              disabled={!debug.console}
              onchange={async (e) => {
                debug.ai = (e.target as HTMLInputElement).checked;
                await savePrefs({ debugAi: debug.ai });
              }}
              class="accent-zinc-400 w-4 h-4 cursor-pointer"
            />
            <div>
              <p class="text-sm text-zinc-200">Se AI-kommunikation</p>
              <p class="text-xs text-zinc-500">Loggar promptar och svar från AI-backend i debug-konsollen.</p>
            </div>
          </label>
        </div>
      {/if}
    </div>
  </div>
</div>
