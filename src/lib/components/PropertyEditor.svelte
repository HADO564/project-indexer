<script lang="ts">
  import { buttonClass, inputClass, labelClass } from "./styles";

  // Free-form key/value facts about a project — "client", "engine",
  // "priority", whatever this set of projects needs. Replaced the single
  // hardcoded `client` field, which only ever suited one kind of user.
  //
  // Edited as a list of rows rather than bound directly to the record, because
  // a half-typed key would otherwise collide with, or wipe, another row's
  // entry on every keystroke.
  let {
    value = $bindable<Record<string, string>>({}),
    // Names already used elsewhere, offered as a datalist so a second project
    // reuses "client" rather than inventing "Client" beside it.
    knownKeys = [],
  }: {
    value?: Record<string, string>;
    knownKeys?: string[];
  } = $props();

  type Row = { key: string; val: string };

  let rows = $state<Row[]>(
    Object.entries(value).map(([key, val]) => ({ key, val })),
  );

  // Rows are the source of truth while editing; `value` is rebuilt from them.
  // A row with a blank name is skipped rather than rejected — it is a row the
  // user has started and not finished, not an error to shout about.
  $effect(() => {
    const next: Record<string, string> = {};
    for (const row of rows) {
      const key = row.key.trim();
      if (key.length === 0) continue;
      next[key] = row.val;
    }
    value = next;
  });

  // Core rejects a name containing ":" because the search bar reads
  // "name: value" and would only ever look up the part before the colon. Say
  // so here rather than letting the save fail.
  const offending = $derived(rows.filter((r) => r.key.includes(":")).map((r) => r.key));
</script>

<div class={labelClass}>
  Properties
  <p class="font-mono text-[12px] normal-case text-phos-faint">
    Searchable as <span class="text-phos-dim">name: value</span> in the search bar.
  </p>

  <div class="flex flex-col gap-1.5">
    {#each rows as row, i (i)}
      <div class="flex items-center gap-1.5">
        <input
          bind:value={row.key}
          placeholder="client"
          aria-label="Property name"
          list="property-names"
          class={`w-1/3 min-w-0 ${inputClass}`}
        />
        <input
          bind:value={row.val}
          placeholder="Acme Corp"
          aria-label="Property value"
          class={`min-w-0 flex-1 ${inputClass}`}
        />
        <button
          type="button"
          onclick={() => (rows = rows.filter((_, n) => n !== i))}
          class="shrink-0 rounded-sm px-2 py-1 font-display text-[15px] leading-none text-phos-faint hover:text-rust"
          title="Remove this property"
          aria-label={`Remove property ${row.key || i + 1}`}
        >
          ×
        </button>
      </div>
    {/each}
  </div>

  <datalist id="property-names">
    {#each knownKeys as key}
      <option value={key}></option>
    {/each}
  </datalist>

  {#if offending.length > 0}
    <p class="text-[12px] normal-case text-rust">
      A property name cannot contain a colon — the search bar reads
      <span class="font-mono">name: value</span>. Rename: {offending.join(", ")}
    </p>
  {/if}

  <button
    type="button"
    onclick={() => (rows = [...rows, { key: "", val: "" }])}
    class={`self-start ${buttonClass}`}
  >
    + Add property
  </button>
</div>
