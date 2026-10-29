import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  Dropped,
  IncomingRequest,
  SelectionInfo,
  Status,
  TransferUpdate,
} from "./api";

type Handler = (payload: never) => void;
const handlers = new Map<string, Handler>();

const api = vi.hoisted(() => ({
  acquireRadio: vi.fn(),
  releaseRadio: vi.fn(),
  getStatus: vi.fn(),
  startBrowse: vi.fn(),
  stopBrowse: vi.fn(),
  setVisibility: vi.fn(),
  respond: vi.fn(),
  openLink: vi.fn(),
  setSettings: vi.fn(),
  sendSelection: vi.fn(),
  sendLink: vi.fn(),
  chooseItems: vi.fn(),
  discardSelection: vi.fn(),
  on: vi.fn(),
}));

vi.mock("./api", async (original) => ({
  ...(await original<typeof import("./api")>()),
  ...api,
}));

import { FINISHED_SHOWN_MS, Model } from "./model.svelte";

const status: Status = {
  name: "bsjubu26",
  interface: "awdl0",
  visibility: "off",
  radio: { state: "idle", held: false },
  receiveDir: "/home/b/Downloads/Owly",
  contactsOnlyAvailable: false,
  browsing: false,
  settings: {
    openLinks: false,
    openLinksFixed: false,
    file: "/home/b/.config/owly/owly_config.toml",
  },
};

function emit(event: string, payload: unknown): void {
  const handler = handlers.get(event);
  if (!handler) throw new Error(`nothing listens to ${event}`);
  handler(payload as never);
}

function request(id: number): IncomingRequest {
  return {
    id,
    senderName: "iPhone",
    senderModel: "iPhone15,2",
    offer: {
      type: "urls",
      links: [
        {
          text: "https://example.org",
          nonAscii: [],
          nonAsciiCount: 0,
          punycode: false,
        },
      ],
      more: 0,
    },
  };
}

function transfer(patch: Partial<TransferUpdate>): TransferUpdate {
  return {
    id: 1,
    peer: "p1",
    state: "waiting",
    sent: 0,
    total: 0,
    reason: null,
    ...patch,
  };
}

beforeEach(() => {
  handlers.clear();
  for (const fn of Object.values(api)) fn.mockReset();
  api.on.mockImplementation((event: string, handler: Handler) => {
    handlers.set(event, handler);
    return Promise.resolve(() => handlers.delete(event));
  });
  api.getStatus.mockResolvedValue(status);
  for (const fn of [
    api.acquireRadio,
    api.releaseRadio,
    api.startBrowse,
    api.stopBrowse,
    api.setVisibility,
    api.respond,
    api.openLink,
    api.setSettings,
    api.sendSelection,
    api.sendLink,
    api.discardSelection,
  ]) {
    fn.mockResolvedValue(undefined);
  }
});

afterEach(() => vi.useRealTimers());

describe("Model", () => {
  it("reads the status when it starts", async () => {
    const model = new Model();
    await model.init();
    expect(model.status?.name).toBe("bsjubu26");
  });

  it("queues incoming requests and drops a withdrawn one", async () => {
    const model = new Model();
    await model.init();
    emit("incoming-request", request(1));
    emit("incoming-request", request(2));
    emit("request-withdrawn", 1);
    expect(model.incoming.map((r) => r.id)).toEqual([2]);
  });

  it("answers a request and removes it", async () => {
    const model = new Model();
    await model.init();
    emit("incoming-request", request(7));
    await model.answer(7, true);
    expect(api.respond).toHaveBeenCalledWith(7, true);
    expect(model.incoming).toEqual([]);
  });

  it("shows only the names of received files", async () => {
    const model = new Model();
    await model.init();
    emit("files-received", { id: 3, paths: ["/home/b/Owly/a.jpg"] });
    expect(model.arrivals).toEqual([
      { key: "files-3", kind: "files", lines: ["a.jpg"] },
    ]);
    model.dismiss("files-3");
    expect(model.arrivals).toEqual([]);
  });

  // spec: OWLY-7
  it("keeps what is odd about each received link and its receipt", async () => {
    const model = new Model();
    await model.init();
    const odd = {
      text: "https://ex\u0430mple.org",
      nonAscii: ["U+0430"],
      nonAsciiCount: 1,
      punycode: false,
    };
    emit("urls-received", {
      id: 5,
      links: [
        {
          text: "https://example.org",
          nonAscii: [],
          nonAsciiCount: 0,
          punycode: false,
        },
        odd,
      ],
      more: 2,
    });
    expect(model.arrivals).toHaveLength(1);
    const arrival = model.arrivals[0];
    if (!arrival) throw new Error("nothing arrived");
    expect(arrival.receipt).toBe(5);
    expect(arrival.links?.[1]).toEqual(odd);
    expect(arrival.more).toBe(2);
  });

  // spec: OWLY-5, OWLY-7
  it("opens a link by receipt and position, never by address", async () => {
    api.openLink.mockResolvedValue(undefined);
    const model = new Model();
    await model.init();
    await model.openLink(5, 1);
    expect(api.openLink).toHaveBeenCalledWith(5, 1);
  });

  it("shows why a link would not open", async () => {
    api.openLink.mockRejectedValue({
      kind: "unavailable",
      message: "opening links is off",
    });
    const model = new Model();
    await model.init();
    await model.openLink(5, 0);
    expect(model.problem).toBe("opening links is off");
  });

  // spec: OWLY-7
  it("asks the backend to turn links on, then reads what is in force", async () => {
    api.setSettings.mockResolvedValue(undefined);
    const model = new Model();
    await model.init();
    api.getStatus.mockClear();
    await model.setOpenLinks(true);
    expect(api.setSettings).toHaveBeenCalledWith(true);
    expect(api.getStatus).toHaveBeenCalledOnce();
  });

  it("clears a finished transfer after a while, but not a newer one", async () => {
    vi.useFakeTimers();
    const model = new Model();
    await model.init();
    emit("transfer", transfer({ id: 1, state: "delivered" }));
    emit("transfer", transfer({ id: 2, state: "waiting" }));
    vi.advanceTimersByTime(FINISHED_SHOWN_MS + 1);
    expect(model.transfers["p1"]?.id).toBe(2);
    emit("transfer", transfer({ id: 2, state: "failed", reason: "x" }));
    vi.advanceTimersByTime(FINISHED_SHOWN_MS + 1);
    expect(model.transfers["p1"]).toBeUndefined();
  });

  describe("turning on", () => {
    it("takes the radio, then looks", async () => {
      const model = new Model();
      expect(await model.turnOn()).toBe(true);
      expect(api.acquireRadio).toHaveBeenCalledOnce();
      expect(api.startBrowse).toHaveBeenCalledOnce();
      expect(model.problem).toBeNull();
    });

    it("stops with the reason when the radio is refused", async () => {
      api.acquireRadio.mockRejectedValue({
        kind: "not_authorized",
        message: "polkit said no",
      });
      const model = new Model();
      expect(await model.turnOn()).toBe(false);
      expect(api.startBrowse).not.toHaveBeenCalled();
      expect(model.problem).toBe("polkit said no");
    });

    it("tries the interface when owlyd isn't there, and says so if that fails", async () => {
      api.acquireRadio.mockRejectedValue({
        kind: "radio_unreachable",
        message: "no owlyd",
      });
      api.startBrowse.mockRejectedValue({
        kind: "io",
        message: "awdl0: no such device",
      });
      const model = new Model();
      expect(await model.turnOn()).toBe(false);
      expect(api.startBrowse).toHaveBeenCalledOnce();
      expect(model.problem).toContain("awdl0: no such device");
      expect(model.problem).toContain("owlyd");
    });
  });

  describe("visibility", () => {
    it("turns the radio on first when it's off", async () => {
      const model = new Model();
      await model.init();
      await model.setVisibility("everyone");
      expect(api.startBrowse).toHaveBeenCalled();
      expect(api.setVisibility).toHaveBeenCalledWith("everyone");
    });

    it("doesn't ask the backend if the radio can't come on", async () => {
      api.acquireRadio.mockRejectedValue({ kind: "busy", message: "in use" });
      const model = new Model();
      await model.init();
      await model.setVisibility("everyone");
      expect(api.setVisibility).not.toHaveBeenCalled();
      expect(model.problem).toBe("in use");
    });

    it("shows the backend's refusal, as for Contacts Only", async () => {
      api.getStatus.mockResolvedValue({ ...status, browsing: true });
      api.setVisibility.mockRejectedValue({
        kind: "unavailable",
        message: "needs an identity",
      });
      const model = new Model();
      await model.init();
      await model.setVisibility("contacts_only");
      expect(model.problem).toBe("needs an identity");
    });

    it("turning off needs no radio", async () => {
      const model = new Model();
      await model.init();
      await model.setVisibility("off");
      expect(api.acquireRadio).not.toHaveBeenCalled();
      expect(api.setVisibility).toHaveBeenCalledWith("off");
    });
  });

  describe("sending", () => {
    it("sends a link by its text", async () => {
      const model = new Model();
      expect(await model.sendLink("p1", "https://example.org")).toBe(true);
      expect(api.sendLink).toHaveBeenCalledWith("p1", "https://example.org");
    });

    it("sends a selection by its id, never by a path", async () => {
      const model = new Model();
      expect(await model.sendSelection("p1", 5)).toBe(true);
      expect(api.sendSelection).toHaveBeenCalledWith("p1", 5);
    });

    it("reports a refusal and says it failed", async () => {
      api.sendSelection.mockRejectedValue({
        kind: "not_found",
        message: "gone",
      });
      const model = new Model();
      expect(await model.sendSelection("p1", 5)).toBe(false);
      expect(model.problem).toBe("gone");
    });
  });

  describe("choosing", () => {
    const picked: SelectionInfo = {
      id: 3,
      count: 1,
      names: ["a.jpg"],
      more: 0,
    };

    it("returns what the picker kept, adding to a selection when given one", async () => {
      api.chooseItems.mockResolvedValue(picked);
      const model = new Model();
      expect(await model.choose(3, false)).toEqual(picked);
      expect(api.chooseItems).toHaveBeenCalledWith(3, false);
    });

    it("gives nothing and the reason when the picker fails", async () => {
      api.chooseItems.mockRejectedValue({
        kind: "invalid",
        message: "not a file",
      });
      const model = new Model();
      expect(await model.choose(null, true)).toBeNull();
      expect(model.problem).toBe("not a file");
    });

    it("lets the backend forget a selection", async () => {
      const model = new Model();
      await model.discard(3);
      expect(api.discardSelection).toHaveBeenCalledWith(3);
    });
  });

  describe("dropping files on the window", () => {
    const dropped: Dropped = {
      selection: { id: 8, count: 1, names: ["a.jpg"], more: 0 },
      x: 10,
      y: 20,
    };

    it("tracks the drag, and clears it on leaving or dropping", async () => {
      const model = new Model();
      await model.init();
      emit("drag-over", { x: 5, y: 6 });
      expect(model.drag).toEqual({ x: 5, y: 6 });
      emit("drag-leave", null);
      expect(model.drag).toBeNull();
      emit("drag-over", { x: 5, y: 6 });
      emit("files-dropped", dropped);
      expect(model.drag).toBeNull();
    });

    it("hands a drop to whoever handles drops", async () => {
      const model = new Model();
      await model.init();
      const seen: Dropped[] = [];
      model.handleDrops((d) => seen.push(d));
      emit("files-dropped", dropped);
      expect(seen).toEqual([dropped]);
      model.handleDrops(null);
      expect(() => emit("files-dropped", dropped)).not.toThrow();
    });
  });
});
