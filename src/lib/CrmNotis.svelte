<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  // Startup-notis: visar alla "återkomst senare"-bolag vars datum har
  // passerat (idag eller tidigare). Öppnas en gång vid appstart om
  // count_crm_forfallna > 0 (se +page.svelte). Per rad finns "Redigera"
  // (öppnar CrmManager på det bolaget) och en "Redigera alla"-knapp som
  // öppnar CrmManager i bläddringsläge med HELA listan.
  // Egen liten centrerad dialog (inte Modal.svelte, som är en fullskärmspanel
  // — opassande stor för en kort notislista), samma overlay-mönster som
  // sackDialog/KontakterPanel:s säck-väljare.
  type CrmBolag = {
    orgnr: string;
    orgnamn: string;
    notis: string;
    aterkom_datum: string | null;
  };

  type Props = {
    onClose: () => void;
    onEdit: (bolag: { orgnr: string; orgnamn: string }[]) => void;
  };
  let { onClose, onEdit }: Props = $props();

  let rows = $state<CrmBolag[]>([]);
  let loading = $state(true);

  async function ladda() {
    loading = true;
    try {
      rows = await invoke<CrmBolag[]>("list_crm_forfallna");
    } finally {
      loading = false;
    }
  }
  ladda();
</script>

<div
  class="fixed inset-0 z-[70] flex items-center justify-center bg-black/50"
  role="dialog"
  aria-modal="true"
  onclick={onClose}
>
  <div
    class="bg-zinc-900 border border-amber-900/60 rounded-xl p-5 w-96 max-h-[70vh] shadow-2xl flex flex-col gap-3"
    onclick={(e) => e.stopPropagation()}
  >
    <div class="flex items-center gap-2">
      <span class="w-2 h-2 rounded-full bg-amber-400 shrink-0"></span>
      <p class="text-sm font-medium text-zinc-100">
        {rows.length} förfallen{rows.length !== 1 ? "a" : ""} återkomst{rows.length !== 1 ? "er" : ""}
      </p>
      {#if rows.length > 1}
        <button
          class="ml-auto text-xs rounded px-2 py-1 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 transition-colors cursor-pointer whitespace-nowrap"
          onclick={() => { onClose(); onEdit(rows.map(r => ({ orgnr: r.orgnr, orgnamn: r.orgnamn }))); }}
        >Redigera alla</button>
      {/if}
    </div>

    <div class="flex-1 overflow-y-auto flex flex-col gap-1.5 -mx-1 px-1">
      {#if loading}
        <p class="text-xs text-zinc-600">Laddar...</p>
      {:else}
        {#each rows as r}
          <div class="rounded-lg bg-zinc-950 border border-zinc-800 px-3 py-2 flex items-center gap-3 group">
            <div class="flex-1 min-w-0">
              <p class="text-sm text-zinc-200 truncate">{r.orgnamn || r.orgnr}</p>
              <p class="text-xs text-zinc-500 truncate">
                Återkom: {r.aterkom_datum}
                {#if r.notis}· {r.notis}{/if}
              </p>
            </div>
            <button
              class="opacity-0 group-hover:opacity-100 text-xs text-zinc-500 hover:text-zinc-200 transition-opacity cursor-pointer px-2 py-1 shrink-0"
              onclick={() => { onClose(); onEdit([{ orgnr: r.orgnr, orgnamn: r.orgnamn }]); }}
            >Redigera</button>
          </div>
        {/each}
      {/if}
    </div>

    <div class="flex justify-end pt-1">
      <button
        class="px-3 py-1.5 text-xs text-zinc-400 hover:text-zinc-200 transition-colors cursor-pointer"
        onclick={onClose}
      >Stäng</button>
    </div>
  </div>
</div>
