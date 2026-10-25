<script lang="ts">
  import CrmTab from "$lib/CrmTab.svelte";

  type Props = { onClose: () => void };
  let { onClose }: Props = $props();
</script>

<div class="flex h-screen bg-zinc-950 text-white flex-col">
  <div class="h-10 shrink-0 flex items-center justify-between px-6 border-b border-zinc-800">
    <span class="text-sm font-medium text-zinc-100">CRM</span>
    <button
      class="text-sm text-zinc-400 hover:text-white transition-colors cursor-pointer"
      onclick={onClose}
    >Stäng</button>
  </div>
  <div class="flex-1 overflow-hidden">
    <CrmTab onClose={() => {}} />
  </div>
</div>
