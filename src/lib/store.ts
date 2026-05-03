import { Store } from "@tauri-apps/plugin-store";

let _store: Store | null = null;

async function getStore(): Promise<Store> {
  if (!_store) _store = await Store.load("prefs.json");
  return _store;
}

export interface Prefs {
  serverUrl: string;
  apiKey: string;
  email: string;
  tier: string;
  launchCount: number;
  dbEtag: string;
  dbSha256: string;
  dbPath: string;
  dbExportDate: string;
  offlineLogins: number;
  aiModel: string;
  geminiApiKey: string;
  geminiModel: string;
  groqApiKey: string;
  groqModel: string;
  aiBackend: string;
  debugConsole: boolean;
  debugAi: boolean;
  smtpHost: string;
  smtpPort: number;
  smtpEncryption: string;
  smtpUsername: string;
  smtpPassword: string;
  smtpFromName: string;
  smtpFromEmail: string;
}

const defaults: Prefs = {
  serverUrl: "",
  apiKey: "",
  email: "",
  tier: "",
  launchCount: 0,
  dbEtag: "",
  dbSha256: "",
  dbPath: "",
  dbExportDate: "",
  offlineLogins: 0,
  aiModel: "",
  geminiApiKey: "",
  geminiModel: "gemini-2.0-flash",
  groqApiKey: "",
  groqModel: "meta-llama/llama-3.3-70b-instruct:free",
  aiBackend: "",
  debugConsole: false,
  debugAi: false,
  smtpHost: "",
  smtpPort: 587,
  smtpEncryption: "starttls",
  smtpUsername: "",
  smtpPassword: "",
  smtpFromName: "",
  smtpFromEmail: "",
};

export async function loadPrefs(): Promise<Prefs> {
  const store = await getStore();
  const prefs: Prefs = { ...defaults };
  for (const key of Object.keys(defaults) as (keyof Prefs)[]) {
    const val = await store.get<Prefs[typeof key]>(key);
    if (val !== null && val !== undefined) (prefs as any)[key] = val;
  }
  return prefs;
}

export async function savePrefs(prefs: Partial<Prefs>): Promise<void> {
  const store = await getStore();
  for (const [key, val] of Object.entries(prefs)) {
    await store.set(key, val);
  }
  await store.save();
}
