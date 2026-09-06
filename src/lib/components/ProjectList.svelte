<script lang="ts">
  import type { Group, Project } from "$lib/api/types";
  import { swatchVar } from "$lib/palette";
  import EditProjectForm from "./EditProjectForm.svelte";
  import ProjectActionsMenu from "./ProjectActionsMenu.svelte";
  import ProjectRow from "./ProjectRow.svelte";
  import { cardClass } from "./styles";

  let {
    projects,
    groups,
    customIcons,
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
    onerror,
  }: {
    projects: Project[];
    groups: Group[];
    customIcons: Map<string, string>;
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
    <p class="text-sm text-phos-dim">No projects yet.</p>
  {:else}
    <ul class="flex flex-col gap-3">
      {#each projects as project (project.id)}
        {@const groupColor = groupColorOf(project)}
        <li
          class="rounded-sm border border-line p-3"
          style={groupColor ? `border-left: 2px solid ${swatchVar(groupColor)}` : undefined}
        >
          {#if editingId === project.id}
            <EditProjectForm {project} {onSaved} onCancel={onCancelEdit} {onerror} />
          {:else}
            <ProjectRow
              {project}
              directoryMissing={missingDirs.has(project.id)}
              {customIcons}
              {groupColor}
            >
              {#snippet actions()}
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
            </ProjectRow>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>
