// What the window knows, and what it does about it. Components read this and
// call its methods; only this file talks to the backend.
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  acquireRadio,
  chooseItems,
  discardSelection,
  errorText,
  getStatus,
  isAppError,
  on,
  openLink,
  releaseRadio,
  respond,
  sendLink,
  sendSelection,
  setSettings,
  setVisibility,
  startBrowse,
  stopBrowse,
  type DragPoint,
  type Dropped,
  type IncomingRequest,
  type Peer,
  type ReceiveFailed,
  type Received,
  type ReceivedLink,
  type ReceivedUrls,
  type SelectionInfo,
  type Status,
  type TransferUpdate,
  type Visibility,
} from "./api";
import { baseName, isFinished } from "./format";

/** Something that arrived, shown until dismissed. */
export interface Arrival {
  key: string;
  kind: "files" | "urls" | "failed";
  lines: string[];
  /** For links: the receipt, the links and how many more were not listed. */
  receipt?: number;
  links?: ReceivedLink[];
  more?: number;
}

/** How long a finished transfer stays under a person's name. */
export const FINISHED_SHOWN_MS = 8000;
const ACTIVITY_KEPT = 50;

export class Model {
  status = $state<Status | null>(null);
  peers = $state<Peer[]>([]);
  problem = $state<string | null>(null);
  transfers = $state<Record<string, TransferUpdate>>({});
  incoming = $state<IncomingRequest[]>([]);
  arrivals = $state<Arrival[]>([]);
  activity = $state<string[]>([]);
  busy = $state(false);
  /** Where files are being dragged over the window, in physical pixels. */
  drag = $state<DragPoint | null>(null);

  #stops: UnlistenFn[] = [];
  #onDrop: ((dropped: Dropped) => void) | null = null;
  #timers = new Set<ReturnType<typeof setTimeout>>();

  /** Subscribes to the backend's events and reads its status. */
  async init(): Promise<void> {
    this.#stops = await Promise.all([
      on<Peer[]>("peers", (found) => (this.peers = found)),
      on<null>("status-changed", () => void this.refresh()),
      on<string>("note", (line) => this.#note(line)),
      on<IncomingRequest>(
        "incoming-request",
        (r) => (this.incoming = [...this.incoming, r]),
      ),
      on<number>("request-withdrawn", (id) => this.#drop(id)),
      on<Received>("files-received", (r) =>
        this.#arrive({
          key: `files-${r.id}`,
          kind: "files",
          lines: r.paths.map(baseName),
        }),
      ),
      on<ReceivedUrls>("urls-received", (r) =>
        this.#arrive({
          key: `urls-${r.id}`,
          kind: "urls",
          lines: r.links.map((l) => l.text),
          receipt: r.id,
          links: r.links,
          more: r.more,
        }),
      ),
      on<ReceiveFailed>("receive-failed", (r) =>
        this.#arrive({
          key: `failed-${r.id}`,
          kind: "failed",
          lines: [r.reason],
        }),
      ),
      on<TransferUpdate>("transfer", (u) => this.#transfer(u)),
      on<DragPoint>("drag-over", (point) => (this.drag = point)),
      on<null>("drag-leave", () => (this.drag = null)),
      on<Dropped>("files-dropped", (dropped) => {
        this.drag = null;
        this.#onDrop?.(dropped);
      }),
    ]);
    await this.refresh();
  }

  /** Sets what to do with files dropped on the window. */
  handleDrops(handler: ((dropped: Dropped) => void) | null): void {
    this.#onDrop = handler;
  }

  dispose(): void {
    for (const stop of this.#stops) stop();
    this.#stops = [];
    for (const timer of this.#timers) clearTimeout(timer);
    this.#timers.clear();
  }

  async refresh(): Promise<void> {
    try {
      this.status = await getStatus();
    } catch (e: unknown) {
      this.problem = errorText(e);
    }
  }

  /** Takes the radio and starts looking. Without owlyd, tries the interface
   * anyway, which is how a test link without a radio works. */
  async turnOn(): Promise<boolean> {
    this.busy = true;
    this.problem = null;
    let withoutOwlyd = false;
    try {
      try {
        await acquireRadio();
      } catch (e: unknown) {
        if (!(isAppError(e) && e.kind === "radio_unreachable")) throw e;
        withoutOwlyd = true;
      }
      try {
        await startBrowse();
      } catch (e: unknown) {
        const hint = withoutOwlyd
          ? " Owly's radio service (owlyd) isn't running."
          : "";
        this.problem = `${errorText(e)}.${hint}`;
        return false;
      }
      return true;
    } catch (e: unknown) {
      this.problem = errorText(e);
      return false;
    } finally {
      this.busy = false;
      await this.refresh();
    }
  }

  async turnOff(): Promise<void> {
    this.busy = true;
    this.problem = null;
    try {
      await setVisibility("off");
      await stopBrowse();
      await releaseRadio();
    } catch (e: unknown) {
      this.problem = errorText(e);
    } finally {
      this.busy = false;
      await this.refresh();
    }
  }

  async setVisibility(mode: Visibility): Promise<void> {
    this.problem = null;
    try {
      if (mode !== "off" && !this.status?.browsing && !(await this.turnOn()))
        return;
      await setVisibility(mode);
    } catch (e: unknown) {
      this.problem = errorText(e);
    } finally {
      await this.refresh();
    }
  }

  /** Sends what the user picked or dropped, which the backend holds. */
  async sendSelection(peer: string, selection: number): Promise<boolean> {
    return this.#sent(() => sendSelection(peer, selection));
  }

  async sendLink(peer: string, url: string): Promise<boolean> {
    return this.#sent(() => sendLink(peer, url));
  }

  async #sent(send: () => Promise<number>): Promise<boolean> {
    this.problem = null;
    try {
      await send();
      return true;
    } catch (e: unknown) {
      this.problem = errorText(e);
      return false;
    }
  }

  /** Opens the native picker for files or a folder, adding to `selection`. */
  async choose(
    selection: number | null,
    directory: boolean,
  ): Promise<SelectionInfo | null> {
    this.problem = null;
    try {
      return await chooseItems(selection, directory);
    } catch (e: unknown) {
      this.problem = errorText(e);
      return null;
    }
  }

  /** Lets the backend forget files that won't be sent. */
  async discard(selection: number): Promise<void> {
    try {
      await discardSelection(selection);
    } catch (e: unknown) {
      this.problem = errorText(e);
    }
  }

  async answer(id: number, accept: boolean): Promise<void> {
    this.#drop(id);
    try {
      await respond(id, accept);
    } catch (e: unknown) {
      this.problem = errorText(e);
    }
  }

  /** Opens a received link in the browser: the backend checks it is on. */
  async openLink(receipt: number, index: number): Promise<void> {
    try {
      await openLink(receipt, index);
    } catch (e: unknown) {
      this.problem = errorText(e);
    }
  }

  /** Turns opening links on or off; turning it on asks in a backend dialog. */
  async setOpenLinks(on: boolean): Promise<void> {
    try {
      await setSettings(on);
    } catch (e: unknown) {
      this.problem = errorText(e);
    }
    await this.refresh();
  }

  dismiss(key: string): void {
    this.arrivals = this.arrivals.filter((a) => a.key !== key);
  }

  #drop(id: number): void {
    this.incoming = this.incoming.filter((r) => r.id !== id);
  }

  #arrive(arrival: Arrival): void {
    this.arrivals = [
      ...this.arrivals.filter((a) => a.key !== arrival.key),
      arrival,
    ];
  }

  #note(line: string): void {
    this.activity = [...this.activity, line].slice(-ACTIVITY_KEPT);
  }

  #transfer(update: TransferUpdate): void {
    this.transfers = { ...this.transfers, [update.peer]: update };
    if (!isFinished(update)) return;
    const timer = setTimeout(() => {
      this.#timers.delete(timer);
      if (this.transfers[update.peer]?.id !== update.id) return;
      const { [update.peer]: _gone, ...rest } = this.transfers;
      this.transfers = rest;
    }, FINISHED_SHOWN_MS);
    this.#timers.add(timer);
  }
}
