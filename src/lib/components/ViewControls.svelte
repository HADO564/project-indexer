<script lang="ts">
  import type { ViewMode } from "$lib/viewState";
  import BundledIcon from "./BundledIcon.svelte";
  import { iconMd, inputClass } from "./styles";

  let {
    mode = $bindable<ViewMode>("list"),
    query = $bindable(""),
  }: {
    mode?: ViewMode;
    query?: string;
  } = $props();

  const modes: { value: ViewMode; icon: string; label: string }[] = [
    { value: "list", icon: "layers", label: "List" },
    { value: "grid", icon: "box", label: "Grid" },
    { value: "compact", icon: "flag", label: "Compact" },
  ];
</script>

<div class="flex items-center gap-2">
  <input
    bind:value={query}
    type="search"
    placeholder="Search name, path or tag"
    aria-label="Search projects"
    class={`h-10 min-w-0 flex-1 ${inputClass}`}
  />
  <div class="flex shrink-0 items-center gap-0.5 rounded-sm border border-line bg-panel-2 p-0.5">
    {#each modes as m}
      <button
        type="button"
        onclick={() => (mode = m.value)}
        aria-pressed={mode === m.value}
        title={m.label}
        aria-label={`${m.label} view`}
        class={`inline-flex h-9 w-9 items-center justify-center rounded-sm ${
          mode === m.value ? "bg-panel text-accent" : "text-phos-dim hover:text-phos"
        }`}
      >
        <BundledIcon name={m.icon} class={iconMd} />
      </button>
    {/each}
  </div>
</div>
