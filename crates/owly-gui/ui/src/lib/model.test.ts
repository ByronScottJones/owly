import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { IncomingRequest, Status, TransferUpdate } from "./api";

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
  sendFiles: vi.fn(),
  sendLinks: vi.fn(),
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
    offer: { type: "urls", urls: ["https://example.org"] },
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
    api.sendFiles,
    api.sendLinks,
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
    it("sends links when given links, else files", async () => {
      const model = new Model();
      expect(await model.send("p1", [], ["https://example.org"])).toBe(true);
      expect(api.sendLinks).toHaveBeenCalledWith("p1", ["https://example.org"]);
      expect(await model.send("p1", ["/a"], [])).toBe(true);
      expect(api.sendFiles).toHaveBeenCalledWith("p1", ["/a"]);
    });

    it("reports a refusal and says it failed", async () => {
      api.sendFiles.mockRejectedValue({
        kind: "invalid",
        message: "not absolute",
      });
      const model = new Model();
      expect(await model.send("p1", ["a"], [])).toBe(false);
      expect(model.problem).toBe("not absolute");
    });
  });
});
