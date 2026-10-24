// A page for looking at the UI without the Tauri backend: development only,
// not part of the build. `npm run dev`, then /mock/mock.html#<scenario>:
// empty, off, people, transfer, incoming, files, doctor, problem.
import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { mount } from "svelte";
import App from "../src/App.svelte";
import "../src/app.css";

const scenario = location.hash.slice(1) || "people";

const people = [
  { id: "a1", name: "Byron Jones Elsevier" },
  { id: "b2", name: "Pixel 9" },
  { id: "c3", name: "Living Room Mac" },
];

const doctorReport = {
  station: "wlp1s0",
  driver: "mt7925e",
  device: "14c3:0717",
  status: "tested",
  mode: "shared",
  channel: 44,
  card: {
    phy: "phy0",
    ibss: true,
    ap: true,
    active_monitor: false,
    monitor_beside_station: true,
    regdom: "DE",
    channels: [{ channel: 6 }, { channel: 149, why: "no-ir" }],
  },
  missing:
    scenario === "problem"
      ? [{ id: "backend", say: "owlyd can't be reached." }]
      : [],
  issues: ["Slow on 2.4 GHz."],
};

mockWindows("main");
mockIPC(
  (cmd) => {
    switch (cmd) {
      case "get_status":
        return {
          name: "Byron Jones",
          interface: "awdl0",
          visibility: scenario === "off" ? "off" : "everyone",
          radio: {
            state: scenario === "off" ? "idle" : "up",
            held: scenario !== "off",
          },
          receiveDir: "/home/byron/Downloads/Owly",
          contactsOnlyAvailable: false,
          browsing: scenario !== "off",
        };
      case "doctor":
        return doctorReport;
      case "plugin:dialog|open":
        return [
          "/home/byron/Pictures/IMG_2777.jpg",
          "/home/byron/Documents/notes.txt",
        ];
      case "send":
        return 1;
      default:
        return null;
    }
  },
  { shouldMockEvents: true },
);

mount(App, { target: document.getElementById("app")! });

// Let the app subscribe before the scenario's events arrive.
setTimeout(async () => {
  if (scenario === "off") return;
  await emit("peers", scenario === "empty" ? [] : people);
  if (scenario === "transfer") {
    await emit("transfer", {
      id: 1,
      peer: "a1",
      state: "progress",
      sent: 40,
      total: 100,
      reason: null,
    });
    await emit("transfer", {
      id: 2,
      peer: "b2",
      state: "failed",
      sent: 0,
      total: 0,
      reason: "timed out",
    });
  }
  if (scenario === "incoming") {
    await emit("incoming-request", {
      id: 9,
      senderName: "Byron's iPhone",
      senderModel: "iPhone15,2",
      offer: {
        type: "files",
        files: [
          { name: "IMG_2777.jpg", kind: "public.jpeg", size: 2570000 },
          { name: "<b>not bold</b>.txt", kind: "public.plain-text", size: 120 },
        ],
      },
    });
  }
  if (scenario === "files") {
    await emit("files-received", {
      id: 4,
      paths: ["/home/byron/Downloads/Owly/IMG_2777.jpg"],
    });
    await emit("urls-received", {
      id: 5,
      urls: ["https://example.org/a-link"],
    });
  }
}, 300);
