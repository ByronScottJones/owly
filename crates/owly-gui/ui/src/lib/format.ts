import type { DoctorReport, ReceivedLink, TransferUpdate } from "./api";

/** Up to two capital letters from a name, for an avatar. */
export function initials(name: string): string {
  const words = name.split(/[\s._-]+/).filter((w) => w.length > 0);
  const letters = words.slice(0, 2).map((w) => Array.from(w)[0] ?? "");
  const text = letters.join("").toLocaleUpperCase();
  return text.length > 0 ? text : "?";
}

/** A stable hue for a name, so a person keeps their colour. */
export function hueOf(name: string): number {
  let hash = 2166136261;
  for (const ch of name) {
    hash = Math.imul(hash ^ ch.codePointAt(0)!, 16777619) >>> 0;
  }
  return hash % 360;
}

const UNITS = ["bytes", "KB", "MB", "GB", "TB"] as const;

/** A file size as people read it; nothing when the sender didn't say. */
export function formatSize(bytes: number | null): string {
  if (bytes === null) return "";
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000;
    unit += 1;
  }
  const digits = unit === 0 || value >= 10 ? 0 : 1;
  return `${value.toLocaleString("en", { maximumFractionDigits: digits })} ${UNITS[unit]}`;
}

/** Whole percent sent, or null when the total isn't known yet. */
export function percent(update: TransferUpdate): number | null {
  if (update.total <= 0) return null;
  return Math.min(100, Math.floor((update.sent / update.total) * 100));
}

/** What to say under a person while a transfer to them runs or ends. */
export function transferLabel(update: TransferUpdate): string {
  switch (update.state) {
    case "waiting":
      return "Waiting…";
    case "accepted":
      return "Accepted";
    case "progress": {
      const done = percent(update);
      return done === null ? "Sending…" : `Sending… ${done}%`;
    }
    case "delivered":
      return "Sent";
    case "declined":
      return "Declined";
    case "failed":
      return update.reason ? `Failed: ${update.reason}` : "Failed";
  }
}

export function isFinished(update: TransferUpdate): boolean {
  return (
    update.state === "delivered" ||
    update.state === "declined" ||
    update.state === "failed"
  );
}

/** The last part of a path, whichever separator it uses. */
export function baseName(path: string): string {
  return (
    path
      .split(/[\\/]/)
      .filter((p) => p.length > 0)
      .pop() ?? path
  );
}

/** The Wi-Fi line of the Doctor panel: `wlp1s0 (mt7925e, 14c3:0717), tested`. */
export function describeStation(report: DoctorReport): string {
  const card = [report.driver, report.device].filter(Boolean).join(", ");
  const station = [report.station, card ? `(${card})` : ""]
    .filter(Boolean)
    .join(" ");
  return report.status ? `${station}, ${report.status}` : station;
}

/** The most different odd characters the backend names (src/model.rs). */
const MAX_ODD_NAMED = 8;

/** The warning for a link with characters outside printable ASCII, or null. */
export function oddCharacters(link: ReceivedLink): string | null {
  if (link.nonAsciiCount === 0) return null;
  const n = link.nonAsciiCount;
  const more = link.nonAscii.length >= MAX_ODD_NAMED ? ", and others" : "";
  return (
    `This address has ${n} character${n === 1 ? "" : "s"} that ` +
    `${n === 1 ? "is" : "are"} not plain ASCII ` +
    `(${link.nonAscii.join(", ")}${more}). It may be imitating another address.`
  );
}
