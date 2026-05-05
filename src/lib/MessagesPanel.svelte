<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  interface Question {
    id: number;
    body: string;
  }

  let {
    serverUrl,
    apiKey,
    canMessage,
    questions,
    onClose,
    onRefresh,
  }: {
    serverUrl: string;
    apiKey: string;
    canMessage: boolean;
    questions: Question[];
    onClose: () => void;
    onRefresh: () => void;
  } = $props();

  let answers: Record<number, string> = $state({});
  let messageBody = $state("");
  let sending = $state<number | "message" | null>(null);
  let sent = $state<Set<number>>(new Set());
  let messageSent = $state(false);
  let error = $state("");

  async function submitAnswer(q: Question) {
    const body = (answers[q.id] ?? "").trim();
    if (!body) return;
    sending = q.id;
    error = "";
    try {
      await invoke("respond_question", { serverUrl, apiKey, questionId: q.id, body });
      sent = new Set([...sent, q.id]);
      onRefresh();
    } catch (e) {
      error = String(e);
    } finally {
      sending = null;
    }
  }

  async function submitMessage() {
    const body = messageBody.trim();
    if (!body) return;
    sending = "message";
    error = "";
    try {
      await invoke("send_message", { serverUrl, apiKey, body });
      messageSent = true;
      messageBody = "";
    } catch (e) {
      error = String(e);
    } finally {
      sending = null;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Backdrop -->
<button
  class="fixed inset-0 z-40"
  onclick={onClose}
  aria-label="Stäng"
  tabindex="-1"
></button>

<!-- Panel -->
<div
  class="fixed top-10 right-2 z-50 w-80 bg-zinc-900 border border-zinc-700 rounded-lg shadow-2xl flex flex-col max-h-[calc(100vh-3rem)] overflow-hidden"
>
  <div class="flex items-center justify-between px-4 py-3 border-b border-zinc-800">
    <span class="text-sm font-medium text-zinc-200">Meddelanden</span>
    <button
      onclick={onClose}
      class="text-zinc-500 hover:text-zinc-200 transition-colors cursor-pointer text-lg leading-none"
      aria-label="Stäng"
    >×</button>
  </div>

  <div class="overflow-y-auto flex-1 p-4 flex flex-col gap-4">
    {#if error}
      <p class="text-xs text-red-400">{error}</p>
    {/if}

    {#each questions as q}
      {#if !sent.has(q.id)}
        <div class="bg-zinc-800 rounded-md p-3 flex flex-col gap-2">
          <p class="text-xs text-zinc-300 leading-relaxed">{q.body}</p>
          <textarea
            bind:value={answers[q.id]}
            placeholder="Ditt svar…"
            rows="2"
            class="w-full bg-zinc-900 border border-zinc-700 rounded text-xs text-zinc-200 placeholder-zinc-600 px-2 py-1.5 resize-none focus:outline-none focus:border-zinc-500"
          ></textarea>
          <button
            onclick={() => submitAnswer(q)}
            disabled={sending === q.id || !(answers[q.id] ?? "").trim()}
            class="self-end px-3 py-1 text-xs bg-white text-zinc-900 font-medium rounded hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
          >
            {sending === q.id ? "Skickar…" : "Skicka svar"}
          </button>
        </div>
      {:else}
        <div class="bg-zinc-800 rounded-md p-3">
          <p class="text-xs text-zinc-500 italic">Svar skickat, tack!</p>
        </div>
      {/if}
    {/each}

    {#if canMessage}
      <div class="flex flex-col gap-2">
        <p class="text-xs text-zinc-500">Förbättringsförslag eller bugg?</p>
        {#if messageSent}
          <p class="text-xs text-green-400">Meddelande skickat, tack!</p>
          <button
            onclick={() => messageSent = false}
            class="self-start text-xs text-zinc-500 hover:text-zinc-300 transition-colors cursor-pointer"
          >Skicka ett till</button>
        {:else}
          <textarea
            bind:value={messageBody}
            placeholder="Skriv ditt meddelande… (max 500 tecken)"
            rows="3"
            maxlength="500"
            class="w-full bg-zinc-800 border border-zinc-700 rounded text-xs text-zinc-200 placeholder-zinc-600 px-2 py-1.5 resize-none focus:outline-none focus:border-zinc-500"
          ></textarea>
          <div class="flex items-center justify-between">
            <span class="text-xs text-zinc-600">{messageBody.length}/500</span>
            <button
              onclick={submitMessage}
              disabled={sending === "message" || !messageBody.trim()}
              class="px-3 py-1 text-xs bg-white text-zinc-900 font-medium rounded hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
            >
              {sending === "message" ? "Skickar…" : "Skicka"}
            </button>
          </div>
        {/if}
      </div>
    {/if}

    {#if questions.length === 0 && !canMessage}
      <p class="text-xs text-zinc-500 text-center py-2">Meddelandegräns nådd, försök igen senare.</p>
    {/if}
  </div>
</div>
