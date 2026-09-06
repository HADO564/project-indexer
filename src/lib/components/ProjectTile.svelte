<script lang="ts">
  import type { Project } from "$lib/api/types";
  import type { Snippet } from "svelte";
  import { swatchVar } from "$lib/palette";
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

<div
  class="flex h-full flex-col gap-2 border-l-2 pl-3"
  style={`border-color: ${groupColor ? swatchVar(groupColor) : "transparent"}`}
>
  <div class="flex items-start justify-between gap-2">
    <ProjectMark {project} {customIcons} size="lg" />
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
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="h-4 w-4"
        >
          <path d="M3 6h18" />
          <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
          <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
          <path d="M10 11v6" />
          <path d="M14 11v6" />
        </svg>
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
