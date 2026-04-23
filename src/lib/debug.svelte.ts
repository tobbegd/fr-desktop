export type LogEntry = { time: string; text: string };

let _console = $state(false);
let _ai = $state(false);
let _logs = $state<LogEntry[]>([]);

export const debug = {
  get console() { return _console; },
  set console(v: boolean) { _console = v; },
  get ai() { return _ai; },
  set ai(v: boolean) { _ai = v; },
  get logs() { return _logs; },
  log(text: string) {
    const time = new Date().toLocaleTimeString("sv-SE");
    _logs = [{ time, text }, ..._logs];
  },
  clear() { _logs = []; },
};
