<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  type CrmBolag = {
    orgnr: string;
    orgnamn: string;
    status: string;
    notis: string;
    aterkom_datum: string | null;
    uppdaterad: string;
  };

  type Props = { onClose: () => void };
  let { onClose }: Props = $props();

  const STATUS_LABELS: Record<string, string> = {
    het: "Het",
    kall: "Kall",
    aterkom: "Återkomst senare",
  };

  // Samma färgval som badgen/indikatorn i sökvyn: het=röd, kall=blå,
  // återkomst=amber (appens befintliga "kräver handling"-färg).
  const STATUS_COLORS: Record<string, { dot: string; text: string; border: string }> = {
    het:     { dot: "bg-red-500",    text: "text-red-400",    border: "border-red-900/60" },
    kall:    { dot: "bg-sky-400",    text: "text-sky-400",    border: "border-sky-900/60" },
    aterkom: { dot: "bg-amber-400",  text: "text-amber-400",  border: "border-amber-900/60" },
  };

  let rows = $state<CrmBolag[]>([]);
  let loading = $state(true);
  let statusFilter = $state<string>("alla");
  let sok = $state("");
  let editOrgnr = $state<string | null>(null);
  let editNotis = $state("");
  let editDatum = $state("");
  let editStatus = $state("het");

  function todayIso(): string {
    return new Date().toISOString().slice(0, 10);
  }

  function isForfallen(r: CrmBolag): boolean {
    return r.status === "aterkom" && !!r.aterkom_datum && r.aterkom_datum <= todayIso();
  }

  async function ladda() {
    loading = true;
    try {
      rows = await invoke<CrmBolag[]>("list_crm_bolag");
    } finally {
      loading = false;
    }
  }

  onMount(() => { ladda(); });

  const filtrerade = $derived.by(() => {
    let r = rows;
    if (statusFilter !== "alla") r = r.filter(x => x.status === statusFilter);
    if (sok.trim()) {
      const q = sok.trim().toLowerCase();
      r = r.filter(x => x.orgnamn.toLowerCase().includes(q) || x.orgnr.includes(q) || x.notis.toLowerCase().includes(q));
    }
    // Förfallna återkomster överst, sedan senast uppdaterad.
    return [...r].sort((a, b) => {
      const af = isForfallen(a) ? 1 : 0;
      const bf = isForfallen(b) ? 1 : 0;
      if (af !== bf) return bf - af;
      return b.uppdaterad.localeCompare(a.uppdaterad);
    });
  });

  const antalForfallna = $derived(rows.filter(isForfallen).length);

  function openEdit(r: CrmBolag) {
    editOrgnr = r.orgnr;
    editStatus = r.status;
    editNotis = r.notis;
    editDatum = r.aterkom_datum ?? "";
  }

  async function saveEdit() {
    if (!editOrgnr) return;
    const r = rows.find(x => x.orgnr === editOrgnr);
    if (!r) return;
    await invoke("set_crm_status", {
      orgnr: editOrgnr,
      orgnamn: r.orgnamn,
      status: editStatus,
      notis: editNotis,
      aterkomDatum: editStatus === "aterkom" && editDatum ? editDatum : null,
    });
    editOrgnr = null;
    await ladda();
  }

  async function removeBolag(orgnr: string) {
    await invoke("remove_crm_bolag", { orgnr });
    await ladda();
  }
</script>

<div class="flex h-full flex-col">
  <div class="px-5 py-3 border-b border-zinc-800 flex items-center gap-3 shrink-0">
    <div>
      <p class="text-sm font-medium text-zinc-200">CRM</p>
      <p class="text-xs text-zinc-500">
        {rows.length} taggade bolag
        {#if antalForfallna > 0}
          <span class="text-amber-400">· {antalForfallna} förfallen{antalForfallna !== 1 ? "a" : ""} återkomst</span>
        {/if}
      </p>
    </div>
    <div class="ml-auto flex items-center gap-2">
      <select
        bind:value={statusFilter}
        class="bg-zinc-900 border border-zinc-800 rounded px-2 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-zinc-600 cursor-pointer"
      >
        <option value="alla">Alla statusar</option>
        <option value="het">Het</option>
        <option value="kall">Kall</option>
        <option value="aterkom">Återkomst senare</option>
      </select>
      <input
        bind:value={sok}
        placeholder="Sök..."
        class="w-40 bg-zinc-900 border border-zinc-800 rounded px-2 py-1.5 text-xs text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600"
      />
    </div>
  </div>

  <div class="flex-1 overflow-y-auto">
    {#if loading}
      <p class="text-xs text-zinc-600 px-5 py-4">Laddar...</p>
    {:else if filtrerade.length === 0}
      <p class="text-xs text-zinc-600 px-5 py-4">
        {rows.length === 0 ? "Inga bolag taggade ännu. Högerklicka på en rad i sökresultaten för att sätta status." : "Inga träffar."}
      </p>
    {:else}
      {#each filtrerade as r}
        {@const colors = STATUS_COLORS[r.status]}
        {@const forfallen = isForfallen(r)}
        <div
          class="flex items-center gap-3 px-5 py-2.5 border-b border-zinc-800/50 bg-zinc-900/40 group
            {forfallen ? 'border-l-2 border-l-amber-400' : ''}"
        >
          <span class="w-2 h-2 rounded-full shrink-0 {colors?.dot ?? 'bg-zinc-600'}"></span>
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <p class="text-sm text-zinc-200 truncate">{r.orgnamn || r.orgnr}</p>
              <span class="text-xs {colors?.text ?? 'text-zinc-500'} shrink-0">{STATUS_LABELS[r.status] ?? r.status}</span>
              {#if forfallen}
                <span class="text-xs text-amber-400 shrink-0">· Förfallen</span>
              {/if}
            </div>
            <p class="text-xs text-zinc-500 truncate">
              {r.orgnr}
              {#if r.aterkom_datum}· Återkom: {r.aterkom_datum}{/if}
              {#if r.notis}· {r.notis}{/if}
            </p>
          </div>
          <button
            class="opacity-0 group-hover:opacity-100 text-xs text-zinc-500 hover:text-zinc-200 transition-opacity cursor-pointer px-2 py-1"
            onclick={() => openEdit(r)}
          >Redigera</button>
          <button
            class="opacity-0 group-hover:opacity-100 text-zinc-600 hover:text-red-400 text-xs transition-opacity cursor-pointer px-1"
            onclick={() => removeBolag(r.orgnr)}
          >✕</button>
        </div>
      {/each}
    {/if}
  </div>
</div>

{#if editOrgnr}
  <div
    class="fixed inset-0 z-[60] flex items-center justify-center bg-black/50"
    role="dialog"
    aria-modal="true"
    onclick={() => editOrgnr = null}
  >
    <div
      class="bg-zinc-900 border border-zinc-700 rounded-xl p-5 w-80 shadow-2xl flex flex-col gap-3"
      onclick={(e) => e.stopPropagation()}
    >
      <p class="text-sm text-zinc-200">Uppdatera status</p>
      <div class="flex gap-1.5">
        {#each ["het", "kall", "aterkom"] as s}
          <button
            class="flex-1 text-xs rounded px-2 py-1.5 border transition-colors cursor-pointer
              {editStatus === s ? (STATUS_COLORS[s].border + ' ' + STATUS_COLORS[s].text + ' bg-zinc-800') : 'border-zinc-700 text-zinc-400 hover:text-zinc-200'}"
            onclick={() => editStatus = s}
          >{STATUS_LABELS[s]}</button>
        {/each}
      </div>
      {#if editStatus === "aterkom"}
        <div>
          <label class="text-xs text-zinc-500 mb-1 block" for="crm-datum">Återkomstdatum</label>
          <input
            id="crm-datum"
            type="date"
            bind:value={editDatum}
            class="w-full bg-zinc-950 border border-zinc-700 rounded px-2 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-zinc-500"
          />
        </div>
      {/if}
      <div>
        <label class="text-xs text-zinc-500 mb-1 block" for="crm-notis">Notis</label>
        <textarea
          id="crm-notis"
          bind:value={editNotis}
          rows="3"
          placeholder="T.ex. Pratade med VD, återkom efter sommaren..."
          class="w-full bg-zinc-950 border border-zinc-700 rounded px-2 py-1.5 text-xs text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-500 resize-none"
        ></textarea>
      </div>
      <div class="flex gap-2 justify-end">
        <button
          class="px-3 py-1.5 text-xs text-zinc-400 hover:text-zinc-200 transition-colors cursor-pointer"
          onclick={() => editOrgnr = null}
        >Avbryt</button>
        <button
          class="px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded hover:bg-zinc-200 transition-colors cursor-pointer"
          onclick={saveEdit}
        >Spara</button>
      </div>
    </div>
  </div>
{/if}
