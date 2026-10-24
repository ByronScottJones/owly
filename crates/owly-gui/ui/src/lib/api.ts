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

export interface Status {
  name: string;
  interface: string;
  visibility: Visibility;
  radio: RadioStatus;
  receiveDir: string;
  contactsOnlyAvailable: boolean;
  browsing: boolean;
}

export interface OfferedFile {
  name: string;
  kind: string;
  size: number | null;
}

export type IncomingOffer =
  { type: "files"; files: OfferedFile[] } | { type: "urls"; urls: string[] };

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

export interface ReceivedUrls {
  id: number;
  urls: string[];
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
export const doctor = (): Promise<string> => invoke<string>("doctor");
export const startBrowse = (): Promise<void> => invoke("start_browse");
export const stopBrowse = (): Promise<void> => invoke("stop_browse");
export const setVisibility = (mode: Visibility): Promise<void> =>
  invoke("set_visibility", { mode });
export const respond = (request: number, accept: boolean): Promise<void> =>
  invoke("respond", { request, accept });

export const sendFiles = (peer: string, paths: string[]): Promise<number> =>
  invoke<number>("send", { peer, paths, urls: [] });
export const sendLinks = (peer: string, urls: string[]): Promise<number> =>
  invoke<number>("send", { peer, paths: [], urls });

/** Subscribes to a backend event; call the result to stop. */
export function on<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  return listen<T>(event, (e) => handler(e.payload));
}
