<script lang="ts">
  import SmtpSetup from "$lib/SmtpSetup.svelte";
  import SackarTab from "$lib/SackarTab.svelte";
  import TemplatesTab from "$lib/TemplatesTab.svelte";
  import UtskickTab from "$lib/UtskickTab.svelte";

  type Props = { onClose: () => void };
  let { onClose }: Props = $props();

  let activeTab = $state("utskick");

  const tabs = [
    { id: "utskick",   label: "Utskick" },
    { id: "sackar",    label: "Säckar" },
    { id: "templates", label: "E-postmallar" },
    { id: "smtp",      label: "Inställningar" },
  ];
</script>

<div class="flex h-screen bg-zinc-950 text-white">
  <!-- Sidebar -->
  <aside class="w-56 shrink-0 border-r border-zinc-800 flex flex-col">
    <div class="h-10 flex items-center px-4 border-b border-zinc-800">
      <span class="text-sm font-medium text-zinc-100">Mailutskick</span>
    </div>
    <nav class="flex-1 p-2">
      {#each tabs as tab}
        <button
          class="w-full text-left px-3 py-2 rounded-md text-sm transition-colors cursor-pointer
            {activeTab === tab.id ? 'bg-zinc-800 text-white' : 'text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200'}"
          onclick={() => activeTab = tab.id}
        >{tab.label}</button>
      {/each}
    </nav>
  </aside>

  <!-- Content -->
  <div class="flex-1 flex flex-col overflow-hidden">
    <div class="h-10 shrink-0 flex items-center justify-between px-6 border-b border-zinc-800">
      <span class="text-sm font-medium text-zinc-100">
        {tabs.find(t => t.id === activeTab)?.label}
      </span>
      <button
        class="text-sm text-zinc-400 hover:text-white transition-colors cursor-pointer"
        onclick={onClose}
      >Stäng</button>
    </div>

    <div class="flex-1 overflow-hidden {activeTab === 'smtp' ? 'overflow-y-auto p-6' : ''}">
      {#if activeTab === "smtp"}
        <SmtpSetup />
      {:else if activeTab === "sackar"}
        <SackarTab />
      {:else if activeTab === "templates"}
        <TemplatesTab />
      {:else if activeTab === "utskick"}
        <UtskickTab />
      {/if}
    </div>
  </div>
</div>
