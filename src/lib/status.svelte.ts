type StatusType = "success" | "error" | "info";

export const status = $state({ message: "", type: "info" as StatusType });

let timer: ReturnType<typeof setTimeout> | null = null;

export function showStatus(msg: string, t: StatusType = "info", duration = 3500) {
  if (timer) clearTimeout(timer);
  status.message = msg;
  status.type = t;
  if (duration > 0) {
    timer = setTimeout(() => { status.message = ""; }, duration);
  }
}

export function clearStatus() {
  if (timer) clearTimeout(timer);
  status.message = "";
}
