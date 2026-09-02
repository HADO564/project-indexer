<script lang="ts">
  import type { Project } from "$lib/api/types";
  import EditProjectForm from "./EditProjectForm.svelte";
  import ProjectCard from "./ProjectCard.svelte";
  import { cardClass } from "./styles";

  let {
    projects,
    loading,
    editingId,
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
  <h2 class="mb-3 text-lg font-semibold text-gray-900 dark:text-gray-100">Projects</h2>
  {#if loading}
    <p class="text-sm text-gray-500 dark:text-gray-400">Loading…</p>
  {:else if projects.length === 0}
    <p class="text-sm text-gray-500 dark:text-gray-400">No projects yet.</p>
  {:else}
    <ul class="flex flex-col gap-3">
      {#each projects as project (project.id)}
        <li class="rounded-md border border-gray-200 p-3 dark:border-gray-700">
          {#if editingId === project.id}
            <EditProjectForm {project} {onSaved} onCancel={onCancelEdit} {onerror} />
          {:else}
            <ProjectCard
              {project}
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
