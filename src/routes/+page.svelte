<script lang="ts">
  import { getAllProjects } from "$lib/api/projects";
  import type { Project } from "$lib/api/types";
  import CreateProjectForm from "$lib/components/CreateProjectForm.svelte";
  import DeleteModal from "$lib/components/DeleteModal.svelte";
  import ErrorBanner from "$lib/components/ErrorBanner.svelte";
  import ProjectList from "$lib/components/ProjectList.svelte";

  let projects = $state<Project[]>([]);
  let loading = $state(false);
  let error = $state("");
  let editingId = $state<string | null>(null);
  let deleteTarget = $state<Project | null>(null);

  async function loadProjects() {
    loading = true;
    error = "";
    try {
      projects = await getAllProjects();
    } catch (err) {
      error = (err as Error).message;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    loadProjects();
  });

  function handleError(message: string) {
    error = message;
  }

  async function handleCreated() {
    error = "";
    await loadProjects();
  }

  function handleEdit(project: Project) {
    editingId = project.id;
    error = "";
  }

  function handleCancelEdit() {
    editingId = null;
  }

  async function handleSaved() {
    editingId = null;
    error = "";
    await loadProjects();
  }

  async function handleOpened() {
    await loadProjects();
  }

  function handleRequestDelete(project: Project) {
    deleteTarget = project;
    error = "";
  }

  function handleCancelDelete() {
    deleteTarget = null;
  }

  async function handleDeleted() {
    if (editingId === deleteTarget?.id) editingId = null;
    deleteTarget = null;
    error = "";
    await loadProjects();
  }
</script>

<main class="mx-auto max-w-3xl px-4 py-8">
  <h1 class="mb-6 text-center text-2xl font-bold text-gray-900 dark:text-gray-100">
    Project Indexer
  </h1>

  <ErrorBanner message={error} />

  <CreateProjectForm onCreated={handleCreated} onerror={handleError} />

  <ProjectList
    {projects}
    {loading}
    {editingId}
    onEdit={handleEdit}
    onCancelEdit={handleCancelEdit}
    onSaved={handleSaved}
    onRequestDelete={handleRequestDelete}
    onOpened={handleOpened}
    onerror={handleError}
  />
</main>

{#if deleteTarget}
  <DeleteModal
    project={deleteTarget}
    onDeleted={handleDeleted}
    onCancel={handleCancelDelete}
    onerror={handleError}
  />
{/if}
