<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { loadPrefs, savePrefs } from "$lib/store";

  let host = $state("");
  let port = $state(587);
  let encryption = $state("starttls");
  let username = $state("");
  let password = $state("");
  let fromName = $state("");
  let fromEmail = $state("");

  const BREVO_URL = "https://www.brevo.com";

  const PRESETS = [
    { label: "Brevo",     host: "smtp-relay.brevo.com",  port: 587, encryption: "starttls", username: "",       hint: "E-postadress som användarnamn, SMTP-nyckel som lösenord (Settings → SMTP & API)", recommended: true },
    { label: "Resend",    host: "smtp.resend.com",        port: 587, encryption: "starttls", username: "resend", hint: "API-nyckel som lösenord",                                                          recommended: false },
    { label: "Mailgun",   host: "smtp.mailgun.org",       port: 587, encryption: "starttls", username: "",       hint: "SMTP-användarnamn + lösenord från Mailgun",                                        recommended: false },
    { label: "SendGrid",  host: "smtp.sendgrid.net",      port: 587, encryption: "starttls", username: "apikey", hint: "API-nyckel som lösenord",                                                          recommended: false },
    { label: "Annan",     host: "",                       port: 587, encryption: "starttls", username: "",       hint: "",                                                                                  recommended: false },
  ];

  function applyPreset(p: typeof PRESETS[0]) {
    host = p.host;
    port = p.port;
    encryption = p.encryption;
    if (p.username) username = p.username;
    presetHint = p.hint;
  }

  let presetHint = $state("");
  let saving = $state(false);
  let saved = $state(false);

  let testTo = $state("");
  let testing = $state(false);
  let testOk = $state(false);
  let testError = $state("");

  loadPrefs().then(p => {
    if (p.smtpHost)       host = p.smtpHost;
    if (p.smtpPort)       port = p.smtpPort;
    if (p.smtpEncryption) encryption = p.smtpEncryption;
    if (p.smtpUsername)   username = p.smtpUsername;
    if (p.smtpPassword)   password = p.smtpPassword;
    if (p.smtpFromName)   fromName = p.smtpFromName;
    if (p.smtpFromEmail)  fromEmail = p.smtpFromEmail;
  });

  async function save() {
    saving = true;
    saved = false;
    await savePrefs({ smtpHost: host, smtpPort: port, smtpEncryption: encryption, smtpUsername: username, smtpPassword: password, smtpFromName: fromName, smtpFromEmail: fromEmail });
    saving = false;
    saved = true;
    setTimeout(() => { saved = false; }, 2000);
  }

  async function sendTest() {
    testing = true;
    testOk = false;
    testError = "";
    try {
      await invoke("send_test_email", {
        host, port, encryption, username, password,
        fromName, fromEmail, toEmail: testTo,
      });
      testOk = true;
    } catch (e) {
      testError = String(e);
    } finally {
      testing = false;
    }
  }
</script>

<div class="flex flex-col gap-6 max-w-md">

  <div class="bg-zinc-900 border border-zinc-800 rounded-lg px-4 py-3 text-xs text-zinc-500 leading-relaxed">
    <p class="text-zinc-400 font-medium mb-1">Ansvar och användning</p>
    Mail skickas via ditt eget SMTP-konto. Du ansvarar för att följa din leverantörs villkor samt gällande lagar
    (t.ex. GDPR och marknadsföringslagen). Rekommendationen är att använda en <span class="text-zinc-300">separat avsändardomän</span> för
    utskick — om den domänen blockeras påverkas inte din huvuddomän.
  </div>

  <div class="flex flex-col gap-2">
    <p class="text-xs text-zinc-500">Leverantör</p>
    <div class="flex gap-2 flex-wrap">
      {#each PRESETS as p}
        <div class="flex flex-col items-start gap-0.5">
          <button onclick={() => applyPreset(p)}
            class="px-3 py-1.5 text-xs rounded-md border transition-colors cursor-pointer
              {host === p.host && p.host !== '' ? 'bg-zinc-700 border-zinc-500 text-white' : 'bg-zinc-900 border-zinc-700 text-zinc-400 hover:text-zinc-200 hover:border-zinc-500'}">
            {p.label}{#if p.recommended} <span class="text-zinc-500">— rekommenderas</span>{/if}
          </button>
          {#if p.recommended}
            <button onclick={() => openUrl(BREVO_URL)} class="text-xs text-zinc-600 hover:text-zinc-400 transition-colors cursor-pointer underline underline-offset-2 px-1">
              Skapa konto hos Brevo
            </button>
          {/if}
        </div>
      {/each}
    </div>
    {#if presetHint}
      <p class="text-xs text-zinc-600">{presetHint}</p>
    {/if}
  </div>

  <div class="flex flex-col gap-3">
    <h2 class="text-sm font-medium text-zinc-200">Server</h2>
    <div class="flex gap-2">
      <div class="flex-1">
        <p class="text-xs text-zinc-500 mb-1">Host</p>
        <input bind:value={host} placeholder="smtp.example.com"
          class="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600" />
      </div>
      <div class="w-24">
        <p class="text-xs text-zinc-500 mb-1">Port</p>
        <input bind:value={port} type="number" placeholder="587"
          class="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600" />
      </div>
    </div>
    <div>
      <p class="text-xs text-zinc-500 mb-1">Kryptering</p>
      <select bind:value={encryption}
        class="w-full appearance-none bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-zinc-600 cursor-pointer">
        <option value="starttls" class="bg-zinc-900 text-zinc-200">STARTTLS (port 587, rekommenderas)</option>
        <option value="tls"      class="bg-zinc-900 text-zinc-200">TLS/SSL (port 465)</option>
        <option value="none"     class="bg-zinc-900 text-zinc-200">Ingen</option>
      </select>
    </div>
  </div>

  <div class="flex flex-col gap-3">
    <h2 class="text-sm font-medium text-zinc-200">Inloggning</h2>
    <div>
      <p class="text-xs text-zinc-500 mb-1">Användarnamn</p>
      <input bind:value={username} placeholder="din@mail.se"
        class="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600" />
    </div>
    <div>
      <p class="text-xs text-zinc-500 mb-1">Lösenord</p>
      <input bind:value={password} type="password" placeholder="••••••••"
        class="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600" />
    </div>
  </div>

  <div class="flex flex-col gap-3">
    <h2 class="text-sm font-medium text-zinc-200">Avsändare</h2>
    <div>
      <p class="text-xs text-zinc-500 mb-1">Namn</p>
      <input bind:value={fromName} placeholder="Mitt Företag"
        class="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600" />
    </div>
    <div>
      <p class="text-xs text-zinc-500 mb-1">E-postadress</p>
      <input bind:value={fromEmail} placeholder="hej@mittforetag.se"
        class="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600" />
    </div>
  </div>

  <button onclick={save} disabled={saving}
    class="w-fit px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded-md hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-50">
    {saving ? "Sparar..." : saved ? "Sparat ✓" : "Spara"}
  </button>

  <div class="border-t border-zinc-800 pt-6 flex flex-col gap-3">
    <h2 class="text-sm font-medium text-zinc-200">Skicka testmail</h2>
    <div class="flex gap-2">
      <input bind:value={testTo} placeholder="mottagare@example.com"
        class="flex-1 bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600" />
      <button onclick={sendTest} disabled={testing || !testTo || !host}
        class="px-3 py-1.5 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 font-medium rounded-md transition-colors cursor-pointer disabled:opacity-50">
        {testing ? "Skickar..." : "Skicka"}
      </button>
    </div>
    {#if testOk}
      <p class="text-xs text-green-400">Testmail skickat.</p>
    {/if}
    {#if testError}
      <p class="text-xs text-red-400">{testError}</p>
    {/if}
  </div>

</div>
