<script lang="ts">
  import type { Project } from "$lib/api/types";
  import EditProjectForm from "./EditProjectForm.svelte";
  import ProjectCard from "./ProjectCard.svelte";
  import { cardClass } from "./styles";

  let {
    projects,
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
        <li class="rounded-sm border border-line p-3">
          {#if editingId === project.id}
            <EditProjectForm {project} {onSaved} onCancel={onCancelEdit} {onerror} />
          {:else}
            <ProjectCard
              {project}
              directoryMissing={missingDirs.has(project.id)}
              {onEdit}
              {onRequestDelete}
              {onOpened}
              {onTrackersRefreshed}
              {onOpenWithAppMissing}
              {onerror}
            />
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>
