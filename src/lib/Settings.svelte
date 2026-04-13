<script lang="ts">
  type Props = {
    serverUrl: string;
    apiKey: string;
    email: string;
    tier: string;
    dbExportDate: string;
    onChangeKey: () => void;
    onClose: () => void;
  };

  let { serverUrl, apiKey, email, tier, dbExportDate, onChangeKey, onClose }: Props = $props();

  let activeSection = $state("general");

  const nav = [
    { id: "general", label: "Allmänt" },
    { id: "auth", label: "Autentisering" },
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
    <div class="h-12 flex items-center justify-between px-6 border-b border-zinc-800">
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

    <div class="flex-1 p-6">
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
      {/if}
    </div>
  </div>
</div>
