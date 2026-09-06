<script lang="ts">
  import type { Group, Project } from "$lib/api/types";
  import { swatchVar } from "$lib/palette";
  import type { ViewMode } from "$lib/viewState";
  import EditProjectForm from "./EditProjectForm.svelte";
  import ProjectActionsMenu from "./ProjectActionsMenu.svelte";
  import ProjectCompactRow from "./ProjectCompactRow.svelte";
  import ProjectRow from "./ProjectRow.svelte";
  import ProjectTile from "./ProjectTile.svelte";
  import { cardClass } from "./styles";

  let {
    projects,
    groups,
    customIcons,
    mode,
    loading,
    editingId,
    missingDirs,
    onEdit,
    onCancelEdit,
    onSaved,
    onRequestDelete,
    onOpened,
    onTrackersRefreshed,
    onOpenWithAppMissing,
    onToggleFavorite,
    emptyMessage = "No projects yet.",
    onerror,
  }: {
    projects: Project[];
    groups: Group[];
    customIcons: Map<string, string>;
    mode: ViewMode;
    loading: boolean;
    editingId: string | null;
    missingDirs: Set<string>;
    onEdit: (project: Project) => void;
    onCancelEdit: () => void;
    onSaved: () => void | Promise<void>;
    onRequestDelete: (project: Project) => void;
    onOpened: () => void | Promise<void>;
    onTrackersRefreshed: () => void | Promise<void>;
    onOpenWithAppMissing: (project: Project) => void;
    // Only the Favourites view supplies this; elsewhere the star stays a
    // static marker, exactly as it is today.
    onToggleFavorite?: (project: Project) => void;
    // Per-view, because "No projects yet." is wrong for a view that filters:
    // you may well have projects and no favourites.
    emptyMessage?: string;
    onerror: (message: string) => void;
  } = $props();

  // The group's palette *name*, resolved per project. Null when the project
  // has no group or when its group has since been deleted.
  function groupColorOf(project: Project): string | null {
    if (!project.group_id) return null;
    return groups.find((g) => g.id === project.group_id)?.color ?? null;
  }
</script>

<section class={cardClass}>
  <h2 class="mb-3 font-display text-[14px] uppercase tracking-wide text-phos-dim"><span class="text-gold">//</span> projects</h2>
  {#if loading && projects.length === 0}
    <!-- Only on a cold start (empty + loading). A re-sort or a create/delete
         refetches too, but it's a local store read — instant — so keep the
         current list on screen rather than flashing this. -->
    <p class="text-sm text-phos-dim">Loading…</p>
  {:else if projects.length === 0}
    <p class="text-sm text-phos-dim">{emptyMessage}</p>
  {:else}
    {#snippet standardActions(project: Project)}
      <ProjectActionsMenu
        {project}
        {onEdit}
        {onRequestDelete}
        {onOpened}
        {onTrackersRefreshed}
        {onOpenWithAppMissing}
        {onerror}
      />
    {/snippet}

    <ul
      class={mode === "grid"
        ? "grid grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-3"
        : mode === "compact"
          ? "flex flex-col"
          : "flex flex-col gap-3"}
    >
      {#each projects as project (project.id)}
        {@const groupColor = groupColorOf(project)}
        {@const missing = missingDirs.has(project.id)}
        <li
          class={mode === "compact"
            ? "border-b border-line px-1 py-1.5 last:border-b-0"
            : "rounded-sm border border-line p-3"}
          style={mode === "list" && groupColor
            ? `border-left: 2px solid ${swatchVar(groupColor)}`
            : undefined}
        >
          {#if editingId === project.id}
            <EditProjectForm {project} {onSaved} onCancel={onCancelEdit} {onerror} />
          {:else if mode === "grid"}
            <ProjectTile
              {project}
              directoryMissing={missing}
              {customIcons}
              {groupColor}
              {onToggleFavorite}
            >
              {#snippet actions()}{@render standardActions(project)}{/snippet}
            </ProjectTile>
          {:else if mode === "compact"}
            <ProjectCompactRow
              {project}
              directoryMissing={missing}
              {customIcons}
              {groupColor}
              {onToggleFavorite}
            >
              {#snippet actions()}{@render standardActions(project)}{/snippet}
            </ProjectCompactRow>
          {:else}
            <ProjectRow
              {project}
              directoryMissing={missing}
              {customIcons}
              {groupColor}
              {onToggleFavorite}
            >
              {#snippet actions()}{@render standardActions(project)}{/snippet}
            </ProjectRow>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>
