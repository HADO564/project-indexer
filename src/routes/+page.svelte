<script lang="ts">
  import { getAllProjects } from "$lib/api/projects";
  import type { Project, SortBy, SortDirection } from "$lib/api/types";
  import BinModal from "$lib/components/BinModal.svelte";
  import CreateProjectForm from "$lib/components/CreateProjectForm.svelte";
  import DeleteModal from "$lib/components/DeleteModal.svelte";
  import ErrorBanner from "$lib/components/ErrorBanner.svelte";
  import FavoritesModal from "$lib/components/FavoritesModal.svelte";
  import OpenWithMissingModal from "$lib/components/OpenWithMissingModal.svelte";
  import ProjectDetailModal from "$lib/components/ProjectDetailModal.svelte";
  import ProjectList from "$lib/components/ProjectList.svelte";
  import SortControls from "$lib/components/SortControls.svelte";

  let projects = $state<Project[]>([]);
  let loading = $state(false);
  let error = $state("");
  let editingId = $state<string | null>(null);
  let deleteTarget = $state<Project | null>(null);
  let binOpen = $state(false);
  let favoritesOpen = $state(false);
  let openWithMissingTarget = $state<Project | null>(null);
  // Stored as an id (not a snapshot) so the modal reflects the latest
  // fetched data if trackers get refreshed while it's open — same
  // convention as `editingId`.
  let detailTargetId = $state<string | null>(null);
  let detailTarget = $derived(projects.find((p) => p.id === detailTargetId) ?? null);
  // Matches the order the main list has always shown by default (most
  // recently opened first) — SortControls lets the user override it.
  let sortBy = $state<SortBy>("last_opened");
  let sortDirection = $state<SortDirection>("descending");

  async function loadProjects() {
    loading = true;
    error = "";
    try {
      projects = await getAllProjects({ by: sortBy, direction: sortDirection });
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

  async function handleTrackersRefreshed() {
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

  function handleOpenBin() {
    binOpen = true;
    error = "";
  }

  function handleCloseBin() {
    binOpen = false;
  }

  async function handleRestored() {
    error = "";
    await loadProjects();
  }

  function handleOpenFavorites() {
    favoritesOpen = true;
    error = "";
  }

  function handleCloseFavorites() {
    favoritesOpen = false;
  }

  async function handleFavoritesChanged() {
    error = "";
    await loadProjects();
  }

  function handleOpenWithAppMissing(project: Project) {
    openWithMissingTarget = project;
    error = "";
  }

  function handleCloseOpenWithMissing() {
    openWithMissingTarget = null;
  }

  function handleShowDetails(project: Project) {
    detailTargetId = project.id;
    error = "";
  }

  function handleCloseDetails() {
    detailTargetId = null;
  }

  async function handleOpenWithMissingResolved() {
    openWithMissingTarget = null;
    error = "";
    await loadProjects();
  }
</script>

<main class="mx-auto max-w-3xl px-4 py-8">
  <div class="mb-6 flex items-center justify-center gap-2">
    <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">Project Indexer</h1>
    <button
      type="button"
      onclick={handleOpenFavorites}
      class="rounded-md p-1.5 text-gray-500 hover:bg-gray-200 hover:text-gray-700 dark:text-gray-400 dark:hover:bg-gray-700 dark:hover:text-gray-200"
      title="Favorites"
      aria-label="Open favorites"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="h-5 w-5"
      >
        <path
          d="M12 3.5l2.6 5.27 5.82.85-4.21 4.1.99 5.79L12 16.9l-5.2 2.61.99-5.79-4.21-4.1 5.82-.85z"
        />
      </svg>
    </button>
    <button
      type="button"
      onclick={handleOpenBin}
      class="rounded-md p-1.5 text-gray-500 hover:bg-gray-200 hover:text-gray-700 dark:text-gray-400 dark:hover:bg-gray-700 dark:hover:text-gray-200"
      title="Bin"
      aria-label="Open bin"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="h-5 w-5"
      >
        <path d="M3 6h18" />
        <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
        <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
        <path d="M10 11v6" />
        <path d="M14 11v6" />
      </svg>
    </button>
  </div>

  <ErrorBanner message={error} />

  <CreateProjectForm onCreated={handleCreated} onerror={handleError} />

  <div class="mb-3 flex justify-end">
    <SortControls bind:by={sortBy} bind:direction={sortDirection} />
  </div>

  <ProjectList
    {projects}
    {loading}
    {editingId}
    onEdit={handleEdit}
    onCancelEdit={handleCancelEdit}
    onSaved={handleSaved}
    onRequestDelete={handleRequestDelete}
    onOpened={handleOpened}
    onTrackersRefreshed={handleTrackersRefreshed}
    onOpenWithAppMissing={handleOpenWithAppMissing}
    onShowDetails={handleShowDetails}
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

{#if binOpen}
  <BinModal onClose={handleCloseBin} onRestored={handleRestored} onerror={handleError} />
{/if}

{#if favoritesOpen}
  <FavoritesModal
    onClose={handleCloseFavorites}
    onChanged={handleFavoritesChanged}
    onOpenWithAppMissing={handleOpenWithAppMissing}
    onerror={handleError}
  />
{/if}

{#if openWithMissingTarget}
  <OpenWithMissingModal
    project={openWithMissingTarget}
    onOpened={handleOpenWithMissingResolved}
    onClose={handleCloseOpenWithMissing}
    onerror={handleError}
  />
{/if}

{#if detailTarget}
  <ProjectDetailModal project={detailTarget} onClose={handleCloseDetails} />
{/if}
