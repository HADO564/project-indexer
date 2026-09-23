<script lang="ts">
  import { listGroups } from "$lib/api/groups";
  import { listCustomIcons } from "$lib/api/icons";
  import {
    getAllProjects,
    getDeletedProjects,
    listMissingDirectories,
    updateProject,
  } from "$lib/api/projects";
  import type { Group, Project, SortBy, SortDirection, View, ViewCounts } from "$lib/api/types";
  import { propertyKeys, resolveView, viewCounts } from "$lib/api/views";
  import { customIconSrc } from "$lib/icons";
  import { buttonClass, primaryButtonClass } from "$lib/components/styles";
  import CreateProjectModal from "$lib/components/CreateProjectModal.svelte";
  import DeleteModal from "$lib/components/DeleteModal.svelte";
  import EditProjectModal from "$lib/components/EditProjectModal.svelte";
  import ErrorBanner from "$lib/components/ErrorBanner.svelte";
  import GroupManagerModal from "$lib/components/GroupManagerModal.svelte";
  import OpenWithMissingModal from "$lib/components/OpenWithMissingModal.svelte";
  import ProjectList from "$lib/components/ProjectList.svelte";
  import ScanFolderModal from "$lib/components/ScanFolderModal.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import SortControls from "$lib/components/SortControls.svelte";
  import ViewControls from "$lib/components/ViewControls.svelte";
  import { latestOnly } from "$lib/latest";
  import { viewKey } from "$lib/views";
  import { loadView, loadViewMode, saveView, saveViewMode, type ViewMode } from "$lib/viewState";

  let projects = $state<Project[]>([]);
  let groups = $state<Group[]>([]);
  // name -> data URI, built once from listCustomIcons and drilled down to
  // ProjectMark. There is no store precedent here — architecture.md lists
  // lib/stores/* as deliberately deferred — so this follows the existing
  // prop-drilling pattern rather than importing a new one.
  let customIcons = $state<Map<string, string>>(new Map());
  let loading = $state(false);
  let error = $state("");
  let editingId = $state<string | null>(null);
  let groupManagerOpen = $state(false);
  let createOpen = $state(false);
  let scanOpen = $state(false);
  let deleteTarget = $state<Project | null>(null);
  let openWithMissingTarget = $state<Project | null>(null);
  let missingDirs = $state<Set<string>>(new Set());
  // Matches the order the main list has always shown by default (most
  // recently opened first) — SortControls lets the user override it.
  let sortBy = $state<SortBy>("last_opened");
  let sortDirection = $state<SortDirection>("descending");

  // Initialised directly rather than in an $effect: +layout.ts sets
  // `ssr = false`, so component init only ever runs in the browser and
  // localStorage is there. The first render is already the restored mode,
  // with no save-effect racing the load.
  let viewMode = $state<ViewMode>(loadViewMode());

  // The selected view cannot be restored the same way: "group:<id>" falls back
  // to All when that group no longer exists, and answering that needs the
  // group list, which has not been fetched at init.
  let groupsLoaded = $state(false);
  let viewRestored = $state(false);

  let selectedView = $state<View>({ kind: "all" });
  let deletedProjects = $state<Project[]>([]);
  let query = $state("");

  // Counts, property names and the search are answered by core
  // (crates/core/src/domain/views.rs) so every frontend agrees on them. They
  // arrive over IPC, which is async, so they are state filled in by the
  // effects below rather than $derived.
  let counts = $state<ViewCounts>({ all: 0, favorites: 0, ungrouped: 0, bin: 0, groups: {} });
  let knownPropertyKeys = $state<string[]>([]);
  // The last answered search, with the view it answered. The list and every
  // view-dependent prop on it read `shown.view`, not `selectedView`, so in the
  // instant between a click and its answer the list is one consistent,
  // slightly old view — never Bin actions on the All list.
  let shown = $state<{ view: View; ids: Set<string> } | null>(null);
  let propertyQuery = $state(false);

  // Resolved from the live list rather than held as a snapshot, so an edit
  // opened before a refetch edits the refetched project. If it disappears —
  // deleted from another window — the modal closes rather than editing a ghost.
  const editingProject = $derived(projects.find((p) => p.id === editingId) ?? null);
  // Filtering the fetched lists, rather than taking projects from the answer,
  // keeps the sort the user chose. Ids are unique across live and binned, so
  // one pass over both is exact.
  const visibleProjects = $derived.by(() => {
    const ids = shown?.ids;
    return ids ? [...projects, ...deletedProjects].filter((p) => ids.has(p.id)) : [];
  });
  const listView = $derived(shown?.view ?? selectedView);

  // Long enough to skip the intermediate keystrokes of a typed word, short
  // enough not to read as lag.
  const SEARCH_DEBOUNCE_MS = 60;
  const searchTicket = latestOnly();
  const countsTicket = latestOnly();
  const keysTicket = latestOnly();
  // What the previous search was dispatched for, so the effect can tell
  // typing (debounced) from a view switch or a refetch (answered at once).
  let lastSearch: { key: string; live: Project[]; binned: Project[] } | null = null;

  async function runSearch(view: View, q: string) {
    const isCurrent = searchTicket();
    try {
      const matches = await resolveView(view, q);
      if (!isCurrent()) return;
      shown = { view, ids: new Set(matches.ids) };
      propertyQuery = matches.property_query;
    } catch (err) {
      if (isCurrent()) error = (err as Error).message;
    }
  }

  // Re-asks whenever the view, the query or the fetched lists change — the
  // lists because an edit, delete or re-sort refetches them, and the answer
  // has to follow. Out-of-order replies are dropped by the ticket.
  $effect(() => {
    const view = $state.snapshot(selectedView);
    const q = query;
    const live = projects;
    const binned = deletedProjects;
    const key = viewKey(view);
    const typing =
      lastSearch !== null &&
      lastSearch.key === key &&
      lastSearch.live === live &&
      lastSearch.binned === binned;
    lastSearch = { key, live, binned };
    const timer = setTimeout(() => void runSearch(view, q), typing ? SEARCH_DEBOUNCE_MS : 0);
    return () => clearTimeout(timer);
  });

  // Counts ignore the query, so they follow the data alone.
  $effect(() => {
    void projects;
    void deletedProjects;
    void groups;
    const isCurrent = countsTicket();
    viewCounts()
      .then((next) => {
        if (isCurrent()) counts = next;
      })
      .catch((err) => {
        if (isCurrent()) error = (err as Error).message;
      });
  });

  // Every property name in use, for the forms' name suggestions and the
  // search hint. Best-effort: a failure only loses the hint.
  $effect(() => {
    void projects;
    void deletedProjects;
    const isCurrent = keysTicket();
    propertyKeys()
      .then((next) => {
        if (isCurrent()) knownPropertyKeys = next;
      })
      .catch(() => {});
  });

  function handleSelectView(view: View) {
    selectedView = view;
    saveView(view);
    error = "";
  }

  // Restore the view exactly once, on the first completed group load. The
  // viewRestored guard is what stops a later refetch — after a group is
  // created or deleted — from yanking the user back to the stored view they
  // have since navigated away from.
  $effect(() => {
    if (groupsLoaded && !viewRestored) {
      selectedView = loadView(groups);
      viewRestored = true;
    }
  });

  $effect(() => {
    saveViewMode(viewMode);
  });

  // A group can disappear underneath the selection — from the group manager,
  // or from another window. Falling back to All beats rendering an empty list
  // with nothing on screen saying why.
  $effect(() => {
    // Read into a local first: `selectedView` is a $state accessor, so
    // TypeScript cannot narrow it across the &&.
    const view = selectedView;
    if (view.kind === "group" && !groups.some((g) => g.id === view.id)) {
      selectedView = { kind: "all" };
    }
  });

  async function loadProjects() {
    loading = true;
    error = "";
    try {
      projects = await getAllProjects({ by: sortBy, direction: sortDirection });
      deletedProjects = await getDeletedProjects({ by: sortBy, direction: sortDirection });
    } catch (err) {
      error = (err as Error).message;
    } finally {
      loading = false;
    }
    // Best-effort marker — a failure here shouldn't block the list.
    try {
      missingDirs = new Set(await listMissingDirectories());
    } catch {
      missingDirs = new Set();
    }
  }

  // Best-effort, both of them: a failure to load groups or custom icons must
  // not stop the project list rendering. No group just means no left edge; a
  // missing custom icon falls back to the bundled glyph.
  async function loadGroups() {
    try {
      groups = await listGroups();
    } catch (err) {
      error = (err as Error).message;
    } finally {
      // Either way the attempt is finished. A failed fetch cannot validate a
      // stored group id, and falling back to All is the right answer there too.
      groupsLoaded = true;
    }
  }

  async function loadCustomIcons() {
    try {
      const icons = await listCustomIcons();
      customIcons = new Map(icons.map((i) => [i.name, customIconSrc(i.svg)]));
    } catch {
      customIcons = new Map();
    }
  }

  $effect(() => {
    loadProjects();
    loadGroups();
    loadCustomIcons();
  });

  async function handleGroupsChanged() {
    error = "";
    await loadGroups();
    // A project's group_id may have been cleared by a group deletion, so the
    // project list is stale too.
    await loadProjects();
  }

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

  // Restore and purge both change which list a project is in, so both refetch.
  async function handleBinChanged() {
    error = "";
    await loadProjects();
  }

  // Only offered in the Favourites view, mirroring the star FavoritesModal
  // had — it and the edit form's checkbox were the only ways to un-favourite,
  // and this view replaces the first of those. A failure still refetches, so
  // the list matches what the backend holds rather than an optimistic guess.
  async function handleToggleFavorite(project: Project) {
    try {
      await updateProject(project.id, { favorite: !project.favorite });
    } catch (err) {
      error = (err as Error).message;
    }
    await loadProjects();
  }

  function handleOpenWithAppMissing(project: Project) {
    openWithMissingTarget = project;
    error = "";
  }

  function handleCloseOpenWithMissing() {
    openWithMissingTarget = null;
  }

  async function handleOpenWithMissingResolved() {
    openWithMissingTarget = null;
    error = "";
    await loadProjects();
  }
</script>

<div class="mx-auto max-w-6xl px-4 py-8">
  <div class="mb-6 flex items-center justify-between gap-2 border-b border-line pb-3">
    <h1 class="text-2xl tracking-wide text-phos">
      <span class="text-accent">&#9612;</span> PROJECT INDEXER
    </h1>
    <div class="flex items-center gap-2">
      <button
        type="button"
        onclick={() => {
          scanOpen = true;
          error = "";
        }}
        class={buttonClass}
      >
        Scan folder
      </button>
      <button
        type="button"
        onclick={() => {
          createOpen = true;
          error = "";
        }}
        class={primaryButtonClass}
      >
        + New project
      </button>
    </div>
  </div>

  <ErrorBanner message={error} />

  <div class="mt-4 flex gap-6">
    <Sidebar
      {groups}
      {counts}
      selected={selectedView}
      onSelect={handleSelectView}
      showFavorites
      showBin
      onManageGroups={() => (groupManagerOpen = true)}
    />

    <main class="min-w-0 flex-1">
      <div class="mb-3 flex items-center gap-2">
        <div class="min-w-0 flex-1">
          <ViewControls bind:mode={viewMode} bind:query {knownPropertyKeys} {propertyQuery} />
        </div>
        <SortControls bind:by={sortBy} bind:direction={sortDirection} />
      </div>

      <ProjectList
        projects={visibleProjects}
        {groups}
        {customIcons}
        mode={viewMode}
        loading={loading || shown === null}
        {missingDirs}
        onEdit={handleEdit}
        onRequestDelete={handleRequestDelete}
        onOpened={handleOpened}
        onTrackersRefreshed={handleTrackersRefreshed}
        onOpenWithAppMissing={handleOpenWithAppMissing}
        onToggleFavorite={listView.kind === "favorites" ? handleToggleFavorite : undefined}
        emptyMessage={listView.kind === "bin"
          ? "The bin is empty."
          : listView.kind === "favorites"
            ? "No favourites yet."
            : "No projects yet."}
        binMode={listView.kind === "bin"}
        onBinChanged={handleBinChanged}
        onerror={handleError}
      />
    </main>
  </div>
</div>

{#if editingProject}
  <EditProjectModal
    project={editingProject}
    {groups}
    {knownPropertyKeys}
    {customIcons}
    onIconsChanged={loadCustomIcons}
    onGroupsStale={loadGroups}
    onSaved={handleSaved}
    onClose={handleCancelEdit}
    onerror={handleError}
  />
{/if}

{#if createOpen}
  <CreateProjectModal
    {groups}
    {knownPropertyKeys}
    {customIcons}
    onIconsChanged={loadCustomIcons}
    onGroupsStale={loadGroups}
    onCreated={handleCreated}
    onClose={() => (createOpen = false)}
    onerror={handleError}
  />
{/if}

{#if scanOpen}
  <ScanFolderModal
    onImported={handleCreated}
    onClose={() => (scanOpen = false)}
    onerror={handleError}
  />
{/if}

{#if groupManagerOpen}
  <GroupManagerModal
    {groups}
    {customIcons}
    onChanged={handleGroupsChanged}
    onClose={() => (groupManagerOpen = false)}
    onerror={handleError}
  />
{/if}

{#if deleteTarget}
  <DeleteModal
    project={deleteTarget}
    onDeleted={handleDeleted}
    onCancel={handleCancelDelete}
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
