<script lang="ts">
  import type { Project } from "$lib/api/types";
  import type { Snippet } from "svelte";
  import BundledIcon from "./BundledIcon.svelte";
  import ProjectMark from "./ProjectMark.svelte";
  import TrackerBadges from "./TrackerBadges.svelte";

  // Grid presentation. Unlike the row, the tile draws its own left edge: it
  // has no <li> border to hang one on.
  let {
    project,
    directoryMissing = false,
    customIcons,
    groupColor = null,
    onToggleFavorite,
    actions,
  }: {
    project: Project;
    directoryMissing?: boolean;
    customIcons: Map<string, string>;
    groupColor?: string | null;
    onToggleFavorite?: (project: Project) => void;
    actions: Snippet;
  } = $props();
</script>

<div class="flex h-full flex-col gap-2">
  <div class="flex items-start justify-between gap-2">
    <ProjectMark {project} {customIcons} {groupColor} size="lg" />
    {@render actions()}
  </div>
  <div class="flex min-w-0 items-center gap-1.5">
    <strong class="min-w-0 truncate font-display text-[15px] text-phos">{project.name}</strong>
    {#if project.favorite}
      {#if onToggleFavorite}
        <button
          type="button"
          onclick={() => onToggleFavorite(project)}
          class="shrink-0 text-gold hover:text-phos"
          title="Remove from favourites"
          aria-label="Remove from favourites"
        >
          ★
        </button>
      {:else}
        <span class="shrink-0 text-gold" title="Favorite">★</span>
      {/if}
    {/if}
    {#if directoryMissing}
      <span class="shrink-0 text-amber" title="Directory deleted or moved">
        <BundledIcon name="trash" class="h-5 w-5" />
      </span>
    {/if}
  </div>
  <div
    class={`truncate text-[11px] ${directoryMissing ? "text-phos-faint line-through" : "text-phos-dim"}`}
    title={project.directory}
  >
    {project.directory}
  </div>
  <div class="mt-auto">
    <TrackerBadges trackers={project.trackers} />
  </div>
</div>
