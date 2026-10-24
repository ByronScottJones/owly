<script lang="ts">
  import { onMount } from "svelte";
  import { doctor, errorText, type DoctorReport } from "./api";
  import { describeStation } from "./format";

  interface Props {
    activity: string[];
  }

  let { activity }: Props = $props();
  let report = $state<DoctorReport | null>(null);
  let problem = $state<string | null>(null);
  let loading = $state(false);

  let ready = $derived(report !== null && (report.missing ?? []).length === 0);

  async function load(): Promise<void> {
    loading = true;
    problem = null;
    try {
      report = await doctor();
    } catch (e: unknown) {
      problem = errorText(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => void load());

  function yesNo(value: boolean | undefined): string {
    return value ? "yes" : "no";
  }
</script>

<div class="doctor">
  <h1>Doctor</h1>
  <p class="quiet">
    What this computer's Wi-Fi card can do for AirDrop, and what stands in the
    way.
  </p>

  <button type="button" class="plain" disabled={loading} onclick={load}>
    {loading ? "Checking…" : "Check again"}
  </button>

  {#if problem}
    <p class="problem" role="alert">{problem}</p>
  {/if}

  {#if report}
    <p class="verdict" role="status">
      <span aria-hidden="true">{ready ? "✓" : "!"}</span>
      {ready ? "Ready for AirDrop." : "Not ready yet."}
    </p>

    {#each report.missing ?? [] as item (item.id)}
      <section class="missing">
        <h2>{item.id}</h2>
        <p>{item.say}</p>
      </section>
    {/each}

    <dl>
      {#if report.station}
        <dt>Wi-Fi</dt>
        <dd>
          {describeStation(report)}
        </dd>
      {/if}
      {#if report.mode}
        <dt>Mode</dt>
        <dd>
          {report.mode}{report.channel ? `, channel ${report.channel}` : ""}
        </dd>
      {/if}
      {#if report.card}
        <dt>Card</dt>
        <dd>
          {report.card.phy ?? "?"}: ad-hoc {yesNo(report.card.ibss)}, AP {yesNo(
            report.card.ap,
          )}, active monitor {yesNo(report.card.active_monitor)}, monitor beside
          the station
          {yesNo(report.card.monitor_beside_station)}
        </dd>
        {#if report.card.regdom}
          <dt>Region</dt>
          <dd>{report.card.regdom}</dd>
        {/if}
        {#if report.card.channels?.length}
          <dt>Channels</dt>
          <dd>
            {report.card.channels
              .map((c) => `${c.channel} ${c.why ?? "ok"}`)
              .join(", ")}
          </dd>
        {/if}
      {/if}
    </dl>

    {#each report.issues ?? [] as issue, i (i)}
      <p class="note">{issue}</p>
    {/each}
  {/if}

  <h2>Activity</h2>
  {#if activity.length > 0}
    <ol class="activity" aria-label="Recent activity">
      {#each activity as line, i (i)}
        <li>{line}</li>
      {/each}
    </ol>
  {:else}
    <p class="quiet">Nothing yet.</p>
  {/if}
</div>

<style>
  .doctor {
    height: 100%;
    padding: 20px 24px;
    overflow: auto;
  }

  h1 {
    font-size: 1.5em;
  }

  h2 {
    margin: 18px 0 6px;
    font-size: 1.1em;
  }

  .quiet {
    color: var(--text-quiet);
  }

  .verdict {
    font-size: 1.2em;
    font-weight: 600;
  }

  .missing {
    margin: 8px 0;
    padding: 4px 14px;
    border: 1px solid var(--danger);
    border-radius: var(--radius);
  }

  .problem {
    color: var(--danger);
  }

  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 16px;
    margin: 12px 0;
  }

  dt {
    color: var(--text-quiet);
  }

  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .activity {
    margin: 0;
    padding-left: 22px;
    color: var(--text-quiet);
    font-size: 0.92em;
    font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
  }
</style>
