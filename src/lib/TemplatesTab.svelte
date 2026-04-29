<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  type Template = { id: number; namn: string; amne: string; brodtext: string; skapad: string };

  let templates = $state<Template[]>([]);
  let selected = $state<Template | null>(null);
  let sparar = $state(false);
  let sparat = $state(false);

  const PLATSHALLARE = ["{{orgnamn}}", "{{orgnr}}", "{{postort}}", "{{bransch}}"];
  const brodtextPlaceholder = "Skriv din text här. Använd platshållare som {{orgnamn}} för personalisering.";

  async function ladda() {
    templates = await invoke<Template[]>("list_templates");
  }

  function valj(t: Template) {
    selected = { ...t };
    sparat = false;
  }

  function nyTemplate() {
    selected = { id: 0, namn: "Ny template", amne: "", brodtext: "", skapad: "" };
    sparat = false;
  }

  async function spara() {
    if (!selected) return;
    sparar = true;
    try {
      if (selected.id === 0) {
        const id = await invoke<number>("create_template", {
          namn: selected.namn,
          amne: selected.amne,
          brodtext: selected.brodtext,
        });
        selected = { ...selected, id };
      } else {
        await invoke("update_template", {
          id: selected.id,
          namn: selected.namn,
          amne: selected.amne,
          brodtext: selected.brodtext,
        });
      }
      await ladda();
      sparat = true;
      setTimeout(() => { sparat = false; }, 2000);
    } finally {
      sparar = false;
    }
  }

  async function taBort(t: Template) {
    await invoke("delete_template", { id: t.id });
    if (selected?.id === t.id) selected = null;
    await ladda();
  }

  function infogaPlatshallare(ph: string) {
    if (!selected) return;
    selected.brodtext += ph;
  }

  onMount(ladda);
</script>

<div class="flex h-full gap-0">
  <!-- Vänster: templatelista -->
  <div class="w-56 shrink-0 border-r border-zinc-800 flex flex-col">
    <div class="p-3 border-b border-zinc-800">
      <button
        onclick={nyTemplate}
        class="w-full px-2 py-1.5 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded transition-colors cursor-pointer text-left"
      >+ Ny template</button>
    </div>
    <div class="flex-1 overflow-y-auto">
      {#each templates as t}
        <div
          role="button"
          tabindex="0"
          class="w-full text-left px-4 py-3 border-b border-zinc-800/60 flex items-center justify-between group transition-colors cursor-pointer
            {selected?.id === t.id ? 'bg-zinc-800' : 'hover:bg-zinc-900'}"
          onclick={() => valj(t)}
          onkeydown={(e) => e.key === "Enter" && valj(t)}
        >
          <p class="text-sm text-zinc-200 truncate pr-2">{t.namn}</p>
          <button
            class="opacity-0 group-hover:opacity-100 text-zinc-600 hover:text-red-400 text-xs transition-all cursor-pointer px-1 shrink-0"
            onclick={(e) => { e.stopPropagation(); taBort(t); }}
          >✕</button>
        </div>
      {:else}
        <p class="text-xs text-zinc-600 px-4 py-3">Inga templates ännu.</p>
      {/each}
    </div>
  </div>

  <!-- Höger: redigeringsformulär -->
  <div class="flex-1 flex flex-col overflow-hidden">
    {#if selected}
      <div class="flex-1 overflow-y-auto p-5 flex flex-col gap-4">
        <div>
          <label class="text-xs text-zinc-500 block mb-1">Namn</label>
          <input
            bind:value={selected.namn}
            class="w-full bg-zinc-900 border border-zinc-800 rounded px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-zinc-600"
          />
        </div>
        <div>
          <label class="text-xs text-zinc-500 block mb-1">Ämnesrad</label>
          <input
            bind:value={selected.amne}
            placeholder="Ämne..."
            class="w-full bg-zinc-900 border border-zinc-800 rounded px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600"
          />
        </div>
        <div class="flex-1 flex flex-col">
          <div class="flex items-center justify-between mb-1">
            <label class="text-xs text-zinc-500">Brödtext</label>
            <div class="flex gap-1">
              {#each PLATSHALLARE as ph}
                <button
                  onclick={() => infogaPlatshallare(ph)}
                  class="text-xs px-1.5 py-0.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-400 hover:text-zinc-200 rounded transition-colors cursor-pointer font-mono"
                >{ph}</button>
              {/each}
            </div>
          </div>
          <textarea
            bind:value={selected.brodtext}
            placeholder={brodtextPlaceholder}
            class="flex-1 min-h-64 w-full bg-zinc-900 border border-zinc-800 rounded px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600 resize-none font-mono leading-relaxed"
          ></textarea>
        </div>
      </div>
      <div class="border-t border-zinc-800 px-5 py-3">
        <button
          onclick={spara}
          disabled={sparar}
          class="px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-50"
        >{sparar ? "Sparar..." : sparat ? "Sparat ✓" : "Spara"}</button>
      </div>
    {:else}
      <div class="flex-1 flex items-center justify-center">
        <p class="text-sm text-zinc-600">Välj en template eller skapa ny</p>
      </div>
    {/if}
  </div>
</div>
