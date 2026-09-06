<script lang="ts">
  import type { Project } from "$lib/api/types";
  import type { Snippet } from "svelte";
  import ProjectMark from "./ProjectMark.svelte";

  // Compact presentation: one dense line. No description, no tags, no tracker
  // badges — the point of this mode is fitting many projects on screen, and
  // the mark plus the name is what you scan for.
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

<div class="flex min-w-0 items-center gap-2">
  <ProjectMark {project} {customIcons} />
  <strong class="shrink-0 truncate font-display text-[14px] text-phos">{project.name}</strong>
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
  <span
    class={`min-w-0 flex-1 truncate text-[11px] ${directoryMissing ? "text-phos-faint line-through" : "text-phos-dim"}`}
    title={project.directory}
  >
    {project.directory}
  </span>
  {@render actions()}
</div>
