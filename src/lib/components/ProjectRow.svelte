<script lang="ts">
  import type { Project } from "$lib/api/types";
  import type { Snippet } from "svelte";
  import ProjectMark from "./ProjectMark.svelte";
  import TrackerBadges from "./TrackerBadges.svelte";

  // List presentation — the card this app has always shown. Its markup is
  // ProjectCard's, unchanged, with the actions menu replaced by a snippet so
  // the Bin can supply its own set.
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
    // The group's palette name, or null when ungrouped or when the group has
    // been deleted. ProjectList draws the left edge on the <li> so the border
    // sits outside this row's padding; the prop is here for symmetry with the
    // tile, which draws its own.
    groupColor?: string | null;
    // When absent the star is a static marker, as it is in every view but
    // Favourites.
    onToggleFavorite?: (project: Project) => void;
    actions: Snippet;
  } = $props();
</script>

<div class="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
  <div class="min-w-0 flex-1">
    <div class="flex min-w-0 items-center gap-2">
      {#if project.icon || project.color}
        <ProjectMark {project} {customIcons} />
      {/if}
      <strong class="min-w-0 truncate font-display text-[15px] text-phos">
        <span class="text-accent">&gt;</span>&nbsp;{project.name}
      </strong>
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
      class={`truncate text-[12px] ${directoryMissing ? "text-phos-faint line-through" : "text-phos-dim"}`}
    >
      {project.directory}
    </div>
    {#if project.description}
      <p class="mt-1 text-sm text-phos-dim">{project.description}</p>
    {/if}
    {#if project.tags.length > 0}
      <div class="mt-2 flex flex-wrap gap-1.5">
        {#each project.tags as tag}
          <span
            class="rounded-sm border border-line px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-phos-dim"
          >
            {tag}
          </span>
        {/each}
      </div>
    {/if}
    <TrackerBadges trackers={project.trackers} />
  </div>
  {@render actions()}
</div>
