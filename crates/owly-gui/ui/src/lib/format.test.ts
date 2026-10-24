import { describe, expect, it } from "vitest";
import type { TransferUpdate } from "./api";
import {
  baseName,
  describeStation,
  formatSize,
  hueOf,
  initials,
  isFinished,
  percent,
  transferLabel,
} from "./format";

function update(patch: Partial<TransferUpdate>): TransferUpdate {
  return {
    id: 1,
    peer: "p",
    state: "waiting",
    sent: 0,
    total: 0,
    reason: null,
    ...patch,
  };
}

describe("initials", () => {
  it("takes the first letters of the first two words", () => {
    expect(initials("Byron Jones Elsevier")).toBe("BJ");
    expect(initials("bsjmac26")).toBe("B");
  });

  it("splits host-name punctuation", () => {
    expect(initials("my-laptop.local")).toBe("ML");
  });

  it("never returns nothing", () => {
    expect(initials("")).toBe("?");
    expect(initials("  ")).toBe("?");
  });
});

describe("hueOf", () => {
  it("is stable and within a circle", () => {
    expect(hueOf("Byron")).toBe(hueOf("Byron"));
    expect(hueOf("Byron")).toBeGreaterThanOrEqual(0);
    expect(hueOf("Byron")).toBeLessThan(360);
  });

  it("tells names apart", () => {
    expect(hueOf("Alice")).not.toBe(hueOf("Bob"));
  });
});

describe("formatSize", () => {
  it("says nothing for an unknown size", () => {
    expect(formatSize(null)).toBe("");
  });

  it("scales units", () => {
    expect(formatSize(0)).toBe("0 bytes");
    expect(formatSize(999)).toBe("999 bytes");
    expect(formatSize(1500)).toBe("1.5 KB");
    expect(formatSize(2_570_000)).toBe("2.6 MB");
    expect(formatSize(25_000_000)).toBe("25 MB");
  });
});

describe("transferLabel", () => {
  it("shows progress as a percentage", () => {
    expect(
      transferLabel(update({ state: "progress", sent: 25, total: 100 })),
    ).toBe("Sending… 25%");
  });

  it("copes with an unknown total", () => {
    expect(
      transferLabel(update({ state: "progress", sent: 25, total: 0 })),
    ).toBe("Sending…");
    expect(percent(update({ sent: 5, total: 0 }))).toBeNull();
  });

  it("never passes 100", () => {
    expect(percent(update({ sent: 120, total: 100 }))).toBe(100);
  });

  it("gives the reason for a failure when there is one", () => {
    expect(
      transferLabel(update({ state: "failed", reason: "timed out" })),
    ).toBe("Failed: timed out");
    expect(transferLabel(update({ state: "failed" }))).toBe("Failed");
  });

  it("knows which states are over", () => {
    expect(isFinished(update({ state: "delivered" }))).toBe(true);
    expect(isFinished(update({ state: "declined" }))).toBe(true);
    expect(isFinished(update({ state: "progress" }))).toBe(false);
  });
});

describe("baseName", () => {
  it("keeps the last part", () => {
    expect(baseName("/home/byron/Owly/photo.jpg")).toBe("photo.jpg");
    expect(baseName("C:\\Users\\b\\a.txt")).toBe("a.txt");
    expect(baseName("/trailing/")).toBe("trailing");
  });
});

describe("describeStation", () => {
  it("joins what is known", () => {
    expect(
      describeStation({
        station: "wlp1s0",
        driver: "mt7925e",
        device: "14c3:0717",
        status: "tested",
      }),
    ).toBe("wlp1s0 (mt7925e, 14c3:0717), tested");
  });

  it("copes with missing parts", () => {
    expect(describeStation({ station: "wlan0" })).toBe("wlan0");
    expect(describeStation({ station: "wlan0", driver: "ath9k" })).toBe(
      "wlan0 (ath9k)",
    );
  });
});
