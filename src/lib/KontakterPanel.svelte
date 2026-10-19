<script lang="ts">
  import { invoke as tauri } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Modal from "./Modal.svelte";

  type Props = { dbPath: string; orgnr: string; orgnamn: string; onclose: () => void };
  let { dbPath, orgnr, orgnamn, onclose }: Props = $props();

  type Kontakt = {
    kategori: string;
    namn: string | null;
    titel: string | null;
    email: string | null;
    telefon: string | null;
    lank: string | null;
    kalla_url: string | null;
  };

  const SOCIAL_KATEGORIER = new Set([
    "instagram", "facebook", "linkedin", "youtube", "x", "tiktok", "pinterest", "bokadirekt",
  ]);

  const SOCIAL_LABELS: Record<string, string> = {
    instagram: "Instagram", facebook: "Facebook", linkedin: "LinkedIn",
    youtube: "YouTube", x: "X", tiktok: "TikTok", pinterest: "Pinterest", bokadirekt: "Bokadirekt",
  };

  let rows = $state<Kontakt[]>([]);
  let loading = $state(true);
  let error = $state("");
  let copiedValue = $state("");

  const personer = $derived(rows.filter(r => r.kategori === "person"));
  const mailboxar = $derived(rows.filter(r => r.kategori === "mailbox"));
  const sociala = $derived(rows.filter(r => SOCIAL_KATEGORIER.has(r.kategori)));

  async function load() {
    loading = true;
    error = "";
    try {
      const res = await tauri<{ columns: string[]; rows: unknown[][] }>("query_db", {
        dbPath,
        sql: `SELECT kategori, namn, titel, email, telefon, lank, kalla_url
              FROM kontakter
              WHERE replace(orgnr, '-', '') = replace('${orgnr}', '-', '')`,
      });
      const cols = res.columns;
      rows = res.rows.map(r => {
        const o: Record<string, unknown> = {};
        cols.forEach((c, i) => o[c] = r[i]);
        return o as unknown as Kontakt;
      });
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => { load(); });

  function copy(v: string) {
    navigator.clipboard.writeText(v);
    copiedValue = v;
    setTimeout(() => { if (copiedValue === v) copiedValue = ""; }, 1200);
  }

  // --- Lägg till i brevsäck, en kontakt i taget ---
  // Varje kontaktrad med email har sin egen knapp. Klick öppnar säck-väljaren
  // för just den emailen. Backend (add_contact_email_to_sack) avgör om det
  // blir bolagets huvudmail (sackar_bolag) eller en extra mottagare
  // (sackar_bolag_extra_email) om bolaget redan finns i säcken med ett annat
  // huvudmail. Ingen flerval-markering behövs — "lägg till" är en engångsaktion.
  type SackItem = { id: number; namn: string; antal: number };
  let pendingEmail = $state<string | null>(null);
  let sackPickerOpen = $state(false);
  let sackarList = $state<SackItem[]>([]);
  let sackNamn = $state("");
  let sackBusy = $state(false);
  let sackResultText = $state("");
  let addedEmails = $state(new Set<string>());

  async function openSackPicker(email: string) {
    pendingEmail = email;
    sackNamn = "";
    sackResultText = "";
    sackarList = await tauri<SackItem[]>("list_sackar");
    sackPickerOpen = true;
  }

  async function addPendingToSack(sackId: number | null) {
    if (!pendingEmail) return;
    sackBusy = true;
    try {
      const id = sackId ?? await tauri<number>("create_sack", { namn: sackNamn.trim() || orgnamn });
      const result = await tauri<"primary" | "extra" | "exists">("add_contact_email_to_sack", {
        sackId: id, orgnr, orgnamn, email: pendingEmail,
      });
      addedEmails = new Set([...addedEmails, pendingEmail]);
      sackResultText = result === "exists" ? "Redan tillagd" : "Tillagd!";
      setTimeout(() => { sackPickerOpen = false; sackResultText = ""; pendingEmail = null; }, 900);
    } finally {
      sackBusy = false;
    }
  }

  function addButtonLabel(email: string): string {
    return addedEmails.has(email) ? "Tillagd ✓" : "Lägg till i brevsäck";
  }
</script>

<Modal title={orgnamn} subtitle={orgnr} {onclose}>
  {#if loading}
    <div class="flex items-center justify-center h-40 text-zinc-500">Laddar...</div>
  {:else if error}
    <div class="text-red-400 text-sm">{error}</div>
  {:else if rows.length === 0}
    <div class="flex items-center justify-center h-40 text-zinc-500">Ingen scrapad kontaktdata hittades.</div>
  {:else}
    <div class="flex flex-col gap-6">
      {#if personer.length > 0}
        <div>
          <div class="text-xs font-medium text-zinc-400 mb-2">Kontaktpersoner ({personer.length})</div>
          <div class="flex flex-col gap-1.5">
            {#each personer as p}
              <div class="flex items-center gap-3 rounded-lg bg-zinc-900 border border-zinc-800 px-3 py-2">
                <div class="flex-1 min-w-0">
                  <div class="text-sm text-zinc-200 truncate">{p.namn ?? "–"}</div>
                  {#if p.titel}<div class="text-xs text-zinc-500 truncate">{p.titel}</div>{/if}
                </div>
                {#if p.email}
                  <button
                    class="text-xs rounded px-2 py-1 transition-colors cursor-pointer
                      {copiedValue === p.email ? 'bg-green-700 text-white' : 'bg-zinc-800 hover:bg-zinc-700 text-zinc-300'}"
                    onclick={() => copy(p.email!)}
                  >{copiedValue === p.email ? "Kopierad!" : p.email}</button>
                {/if}
                {#if p.telefon}
                  <button
                    class="text-xs rounded px-2 py-1 transition-colors cursor-pointer
                      {copiedValue === p.telefon ? 'bg-green-700 text-white' : 'bg-zinc-800 hover:bg-zinc-700 text-zinc-300'}"
                    onclick={() => copy(p.telefon!)}
                  >{copiedValue === p.telefon ? "Kopierad!" : p.telefon}</button>
                {/if}
                {#if p.email}
                  <button
                    class="text-xs rounded px-2 py-1 transition-colors cursor-pointer shrink-0
                      {addedEmails.has(p.email) ? 'bg-green-800 text-green-200' : 'bg-amber-700 hover:bg-amber-600 text-white'}"
                    onclick={() => openSackPicker(p.email!)}
                  >{addButtonLabel(p.email)}</button>
                {/if}
                {#if p.kalla_url}
                  <a
                    href={p.kalla_url}
                    target="_blank" rel="noopener noreferrer"
                    class="text-xs text-zinc-600 hover:text-zinc-400 shrink-0"
                    title="Hittad på {p.kalla_url}"
                  >🔗</a>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}

      {#if mailboxar.length > 0}
        <div>
          <div class="text-xs font-medium text-zinc-400 mb-2">Generella adresser ({mailboxar.length})</div>
          <div class="flex flex-col gap-1.5">
            {#each mailboxar as m}
              <div class="flex items-center gap-3 rounded-lg bg-zinc-900 border border-zinc-800 px-3 py-2">
                <div class="flex-1 min-w-0 text-sm text-zinc-200 truncate">{m.email ?? m.telefon ?? "–"}</div>
                {#if m.email}
                  <button
                    class="text-xs rounded px-2 py-1 transition-colors cursor-pointer
                      {copiedValue === m.email ? 'bg-green-700 text-white' : 'bg-zinc-800 hover:bg-zinc-700 text-zinc-300'}"
                    onclick={() => copy(m.email!)}
                  >{copiedValue === m.email ? "Kopierad!" : "Kopiera"}</button>
                {/if}
                {#if m.telefon}
                  <button
                    class="text-xs rounded px-2 py-1 transition-colors cursor-pointer
                      {copiedValue === m.telefon ? 'bg-green-700 text-white' : 'bg-zinc-800 hover:bg-zinc-700 text-zinc-300'}"
                    onclick={() => copy(m.telefon!)}
                  >{copiedValue === m.telefon ? "Kopierad!" : m.telefon}</button>
                {/if}
                {#if m.email}
                  <button
                    class="text-xs rounded px-2 py-1 transition-colors cursor-pointer shrink-0
                      {addedEmails.has(m.email) ? 'bg-green-800 text-green-200' : 'bg-amber-700 hover:bg-amber-600 text-white'}"
                    onclick={() => openSackPicker(m.email!)}
                  >{addButtonLabel(m.email)}</button>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}

      {#if sociala.length > 0}
        <div>
          <div class="text-xs font-medium text-zinc-400 mb-2">Sociala länkar ({sociala.length})</div>
          <div class="flex flex-wrap gap-2">
            {#each sociala as s}
              {#if s.lank}
                <a
                  href={s.lank}
                  target="_blank" rel="noopener noreferrer"
                  class="text-xs rounded-full px-3 py-1.5 bg-zinc-900 border border-zinc-800 hover:border-zinc-600 text-zinc-300 hover:text-white transition-colors"
                >{SOCIAL_LABELS[s.kategori] ?? s.kategori}</a>
              {/if}
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</Modal>

{#if sackPickerOpen}
  <div
    class="fixed inset-0 z-[60] flex items-center justify-center bg-black/50"
    role="dialog"
    aria-modal="true"
    onclick={() => { if (!sackBusy) { sackPickerOpen = false; pendingEmail = null; } }}
  >
    <div
      class="bg-zinc-900 border border-zinc-700 rounded-xl p-5 w-80 shadow-2xl"
      onclick={(e) => e.stopPropagation()}
    >
      {#if sackResultText}
        <p class="text-sm text-green-400">{sackResultText}</p>
      {:else}
        <p class="text-sm text-zinc-200 mb-1">Lägg till {orgnamn} i brevsäck</p>
        <p class="text-xs text-zinc-500 mb-3 truncate">{pendingEmail}</p>
        {#if sackarList.length > 0}
          <div class="flex flex-col gap-1 mb-3 max-h-40 overflow-y-auto">
            {#each sackarList as s}
              <button
                class="text-left text-xs rounded px-2 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition-colors cursor-pointer disabled:opacity-40"
                disabled={sackBusy}
                onclick={() => addPendingToSack(s.id)}
              >{s.namn} ({s.antal})</button>
            {/each}
          </div>
          <div class="text-xs text-zinc-600 mb-2">eller skapa ny:</div>
        {/if}
        <div class="flex gap-2">
          <input
            bind:value={sackNamn}
            placeholder="Namn på ny säck..."
            onkeydown={(e) => e.key === "Enter" && addPendingToSack(null)}
            class="flex-1 bg-zinc-950 border border-zinc-700 rounded px-2 py-1.5 text-xs text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-500"
          />
          <button
            class="px-2.5 py-1.5 text-xs bg-zinc-700 hover:bg-zinc-600 text-zinc-200 rounded transition-colors cursor-pointer disabled:opacity-40"
            disabled={sackBusy}
            onclick={() => addPendingToSack(null)}
          >Skapa</button>
        </div>
      {/if}
    </div>
  </div>
{/if}
