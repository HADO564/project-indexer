<script lang="ts">
  import type { SortBy, SortDirection } from "$lib/api/types";
  import { inputClass } from "./styles";

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
  <select bind:value={by} class={`${inputClass} py-1.5`} aria-label="Sort by">
    <option value="alphabetical">Name</option>
    <option value="last_opened">Last opened</option>
  </select>
  <button
    type="button"
    onclick={toggleDirection}
    class="rounded-md border border-gray-300 px-2.5 py-1.5 text-sm text-gray-700 hover:bg-gray-100 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
    title={direction === "ascending" ? "Ascending" : "Descending"}
    aria-label="Toggle sort direction"
  >
    {direction === "ascending" ? "↑" : "↓"}
  </button>
</div>
