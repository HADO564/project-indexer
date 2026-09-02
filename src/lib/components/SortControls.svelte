<script lang="ts">
  import type { SortBy, SortDirection } from "$lib/api/types";

  let {
    by = $bindable<SortBy>("alphabetical"),
    direction = $bindable<SortDirection>("ascending"),
  }: {
    by?: SortBy;
    direction?: SortDirection;
  } = $props();

  function toggleDirection() {
    direction = direction === "ascending" ? "descending" : "ascending";
  }
</script>

<div class="flex items-center gap-2">
  <select
    bind:value={by}
    class="h-8 rounded-sm border border-line bg-panel-2 px-2.5 font-mono text-[13px] text-phos focus:border-phos focus:outline-none"
    aria-label="Sort by"
  >
    <option value="alphabetical">Name</option>
    <option value="last_opened">Last opened</option>
  </select>
  <button
    type="button"
    onclick={toggleDirection}
    class="inline-flex h-8 w-8 items-center justify-center rounded-sm border border-line bg-panel-2 font-display text-[16px] leading-none text-phos-dim hover:border-phos hover:text-phos"
    title={direction === "ascending" ? "Ascending" : "Descending"}
    aria-label="Toggle sort direction"
  >
    {direction === "ascending" ? "↑" : "↓"}
  </button>
</div>
