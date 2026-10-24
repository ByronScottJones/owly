<script lang="ts">
  import { onMount } from "svelte";
  import type { Peer } from "./lib/api";
  import AirDropView from "./lib/AirDropView.svelte";
  import Arrivals from "./lib/Arrivals.svelte";
  import DoctorPanel from "./lib/DoctorPanel.svelte";
  import IncomingDialog from "./lib/IncomingDialog.svelte";
  import { Model } from "./lib/model.svelte";
  import SendDialog from "./lib/SendDialog.svelte";

  const model = new Model();
  let view = $state<"airdrop" | "doctor">("airdrop");
  let sending = $state<Peer | null>(null);

  onMount(() => {
    void model.init();
    return () => model.dispose();
  });
</script>

<div class="app">
  <nav aria-label="Sections">
    <ul>
      <li>
        <button
          type="button"
          aria-current={view === "airdrop" ? "page" : undefined}
          onclick={() => (view = "airdrop")}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <circle cx="12" cy="12" r="2.4" fill="currentColor" />
            <path
              d="M7.4 7.4a6.5 6.5 0 0 0 0 9.2M16.6 7.4a6.5 6.5 0 0 1 0 9.2M4.6 4.6a10.5 10.5 0 0 0 0 14.8M19.4 4.6a10.5 10.5 0 0 1 0 14.8"
              fill="none"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
            />
          </svg>
          AirDrop
        </button>
      </li>
      <li>
        <button
          type="button"
          aria-current={view === "doctor" ? "page" : undefined}
          onclick={() => (view = "doctor")}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path
              d="M3 12h4l2.5-6 4 12 2.5-6H21"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
            />
          </svg>
          Doctor
        </button>
      </li>
    </ul>
  </nav>

  <main>
    {#if model.problem}
      <p class="problem" role="alert">
        {model.problem}
        <button
          type="button"
          class="plain"
          onclick={() => (model.problem = null)}>Dismiss</button
        >
      </p>
    {/if}
    {#if view === "airdrop"}
      <AirDropView {model} onchoose={(peer) => (sending = peer)} />
    {:else}
      <DoctorPanel activity={model.activity} />
    {/if}
  </main>
</div>

<IncomingDialog
  request={model.incoming[0]}
  queued={Math.max(0, model.incoming.length - 1)}
  onanswer={(id, accept) => model.answer(id, accept)}
/>
<SendDialog {model} peer={sending} onclose={() => (sending = null)} />
<Arrivals
  arrivals={model.arrivals}
  receiveDir={model.status?.receiveDir ?? ""}
  ondismiss={(key) => model.dismiss(key)}
/>

<style>
  .app {
    display: grid;
    grid-template-columns: 200px 1fr;
    height: 100%;
  }

  nav {
    padding: 16px 10px 10px;
    background: var(--bg-side);
    border-right: 1px solid var(--line);
  }

  nav ul {
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  nav button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border: 0;
    border-radius: 8px;
    background: none;
    font-size: 1.05em;
    text-align: left;
    cursor: pointer;
  }

  nav button[aria-current="page"] {
    background: var(--select-selected);
    font-weight: 600;
  }

  nav svg {
    width: 22px;
    height: 22px;
    color: var(--text-quiet);
  }

  main {
    position: relative;
    min-width: 0;
    height: 100%;
  }

  .problem {
    position: absolute;
    top: 8px;
    left: 24px;
    right: 24px;
    z-index: 4;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin: 0;
    padding: 8px 12px;
    border: 1px solid var(--danger);
    border-radius: var(--radius);
    background: var(--bg-raised);
    color: var(--danger);
  }

  @media (max-width: 720px) {
    .app {
      grid-template-columns: 64px 1fr;
    }

    nav button {
      justify-content: center;
      font-size: 0;
    }
  }
</style>
