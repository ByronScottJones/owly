// The typed face of the backend's commands and events (src/commands.rs,
// src/model.rs). Nothing else in the UI calls `invoke` or `listen`.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Visibility = "everyone" | "contacts_only" | "off";

export interface Peer {
  id: string;
  name: string;
}

export interface RadioStatus {
  state: string;
  held: boolean;
}

export interface SettingsView {
  openLinks: boolean;
  /** OWLY_OPEN_LINKS is set, so the preferences can't change it. */
  openLinksFixed: boolean;
  file: string;
}

export interface Status {
  name: string;
  interface: string;
  visibility: Visibility;
  radio: RadioStatus;
  receiveDir: string;
  contactsOnlyAvailable: boolean;
  browsing: boolean;
  settings: SettingsView;
}

export interface OfferedFile {
  name: string;
  kind: string;
  size: number | null;
}

/** `more` counts what the backend left out of a long offer. */
export type IncomingOffer =
  | { type: "files"; files: OfferedFile[]; more: number }
  | { type: "urls"; urls: string[]; more: number };

/** What the user picked or dropped. The backend keeps the paths: the page
 * only ever has this, and names it by `id`. */
export interface SelectionInfo {
  id: number;
  count: number;
  names: string[];
  more: number;
}

export interface Dropped {
  selection: SelectionInfo;
  x: number;
  y: number;
}

export interface DragPoint {
  x: number;
  y: number;
}

export interface IncomingRequest {
  id: number;
  senderName: string;
  senderModel: string;
  offer: IncomingOffer;
}

export type TransferState =
  "waiting" | "accepted" | "progress" | "delivered" | "declined" | "failed";

export interface TransferUpdate {
  id: number;
  peer: string;
  state: TransferState;
  sent: number;
  total: number;
  reason: string | null;
}

export interface Received {
  id: number;
  paths: string[];
}

/** A received link, and what is odd about it (src/model.rs `ReceivedLink`). */
export interface ReceivedLink {
  text: string;
  /** Code points like "U+0430" of characters that are not printable ASCII. */
  nonAscii: string[];
  nonAsciiCount: number;
}

export interface ReceivedUrls {
  id: number;
  links: ReceivedLink[];
  more: number;
}

export interface ReceiveFailed {
  id: number;
  reason: string;
}

/** What a failed command rejects with. */
export interface AppError {
  kind: string;
  message: string;
}

export function isAppError(e: unknown): e is AppError {
  return (
    typeof e === "object" &&
    e !== null &&
    typeof (e as AppError).kind === "string" &&
    typeof (e as AppError).message === "string"
  );
}

/** The message to show for anything a command or this code threw. */
export function errorText(e: unknown): string {
  if (isAppError(e)) return e.message;
  return e instanceof Error ? e.message : String(e);
}

export const getStatus = (): Promise<Status> => invoke<Status>("get_status");
export const acquireRadio = (): Promise<void> => invoke("acquire_radio");
export const releaseRadio = (): Promise<void> => invoke("release_radio");
export const doctor = (): Promise<DoctorReport> =>
  invoke<DoctorReport>("doctor");
export const startBrowse = (): Promise<void> => invoke("start_browse");
export const stopBrowse = (): Promise<void> => invoke("stop_browse");
export const setVisibility = (mode: Visibility): Promise<void> =>
  invoke("set_visibility", { mode });
export const respond = (request: number, accept: boolean): Promise<void> =>
  invoke("respond", { request, accept });
/** Opens a received link by its receipt and position (OWLY-7). */
export const openLink = (id: number, index: number): Promise<void> =>
  invoke("open_link", { id, index });
export const setSettings = (openLinks: boolean): Promise<void> =>
  invoke("set_settings", { openLinks });

/** Opens the native picker; adds to `selection` when given. Null if cancelled. */
export const chooseItems = (
  selection: number | null,
  directory: boolean,
): Promise<SelectionInfo | null> =>
  invoke<SelectionInfo | null>("choose_items", { selection, directory });
export const discardSelection = (selection: number): Promise<void> =>
  invoke("discard_selection", { selection });
export const sendSelection = (
  peer: string,
  selection: number,
): Promise<number> => invoke<number>("send_selection", { peer, selection });
export const sendLink = (peer: string, url: string): Promise<number> =>
  invoke<number>("send_link", { peer, url });

/** Subscribes to a backend event; call the result to stop. */
export function on<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  return listen<T>(event, (e) => handler(e.payload));
}

/** One thing owly doctor found missing, and what to do about it. */
export interface Missing {
  id: string;
  say: string;
}

export interface CardChannel {
  channel: number;
  why?: string;
}

/** `owly doctor --json`: only the fields the Doctor panel shows. */
export interface DoctorReport {
  station?: string | null;
  driver?: string | null;
  device?: string | null;
  status?: string | null;
  mode?: string | null;
  channel?: number | null;
  card?: {
    phy?: string;
    ibss?: boolean;
    ap?: boolean;
    active_monitor?: boolean;
    monitor_beside_station?: boolean;
    regdom?: string;
    channels?: CardChannel[];
  } | null;
  missing?: Missing[];
  issues?: string[];
}
