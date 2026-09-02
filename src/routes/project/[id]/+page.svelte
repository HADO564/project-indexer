<script lang="ts">
  import { page } from "$app/stores";
  import { isOpenWithAppMissing, openProjectDirectory } from "$lib/api/opener";
  import { inspectProject, refreshProjectTrackers } from "$lib/api/projects";
  import type { ProjectInspection } from "$lib/api/types";
  import EditProjectForm from "$lib/components/EditProjectForm.svelte";
  import ErrorBanner from "$lib/components/ErrorBanner.svelte";
  import ProjectIdentity from "$lib/components/ProjectIdentity.svelte";
  import TrackerPanel from "$lib/components/TrackerPanel.svelte";
  import { buttonClass } from "$lib/components/styles";
  import { trackerKind } from "$lib/trackers";

  // Route always supplies `id`; `?? ""` only satisfies the `string | undefined`
  // param type. `load()` bails on an empty id.
  let id = $derived($page.params.id ?? "");

  let inspection = $state<ProjectInspection | null>(null);
  let loadError = $state("");
  let banner = $state("");
  let loading = $state(false);
  let editing = $state(false);
  let activeKind = $state<string | null>(null);

  async function load(only?: string) {
    if (!id) return;
    loading = true;
    banner = "";
    try {
      const next = await inspectProject(id, only ? { only } : undefined);
      if (only && inspection) {
        // merge a single re-detected result
        const merged = inspection.results.map((r) =>
          r.kind === only ? next.results.find((n) => n.kind === only) ?? r : r,
        );
        inspection = { ...next, results: merged };
      } else {
        inspection = next;
      }
      loadError = "";
    } catch (err) {
      // A per-tab re-detect (`only` set) that fails must not collapse the
      // whole view — leave `inspection` intact and surface it in the banner.
      // Only a full load writes `loadError`.
      if (only) {
        banner = (err as Error).message;
      } else {
        loadError = (err as Error).message;
      }
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    // re-runs when `id` changes
    void id;
    load();
  });

  let detected = $derived(inspection?.results.filter((r) => r.status === "detected") ?? []);

  $effect(() => {
    if (detected.length > 0 && !detected.some((r) => r.kind === activeKind)) {
      activeKind = detected[0].kind;
    }
  });

  async function handleOpen() {
    try {
      await openProjectDirectory(id);
      await load();
    } catch (err) {
      banner = isOpenWithAppMissing(err)
        ? "The app configured for this project can't be found."
        : (err as Error).message;
    }
  }

  async function handleRefresh() {
    // Set `loading` up front so the button's `disabled={loading}` covers the
    // `refreshProjectTrackers` await too — a fast double-click can't double-fire.
    loading = true;
    let refreshError = "";
    try {
      await refreshProjectTrackers(id);
    } catch (err) {
      refreshError = (err as Error).message;
    }
    // Repaint from a fresh inspect regardless; load() clears `banner` and
    // manages `loading` from here, so re-apply the refresh error afterwards.
    await load();
    if (refreshError) banner = refreshError;
  }

  async function handleSaved() {
    editing = false;
    await load();
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (editing && e.key === "Escape") editing = false;
  }}
/>

<main class="mx-auto max-w-3xl px-4 py-8">
  <a href="/" class="text-sm text-blue-600 hover:underline dark:text-blue-400">← All projects</a>

  {#if loadError}
    <div class="mt-4">
      <ErrorBanner message={loadError} />
    </div>
  {:else if inspection}
    <div class="mt-3 flex items-start justify-between gap-4">
      <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">
        {inspection.project.name}
      </h1>
      <div class="flex shrink-0 gap-2">
        <button type="button" class={buttonClass} onclick={handleOpen}>Open</button>
        <button type="button" class={buttonClass} onclick={() => (editing = true)}>Edit</button>
        <button type="button" class={buttonClass} onclick={handleRefresh} disabled={loading}>
          {loading ? "…" : "Refresh"}
        </button>
      </div>
    </div>

    <div class="mt-3"><ErrorBanner message={banner} /></div>

    <div class="mt-2 rounded-lg bg-white p-4 shadow-sm dark:bg-gray-800">
      <ProjectIdentity project={inspection.project} />
    </div>

    {#if !inspection.directory_status.ok}
      <p class="mt-4 rounded-md bg-amber-100 p-3 text-sm text-amber-900 dark:bg-amber-950 dark:text-amber-200">
        {inspection.directory_status.message ?? "This project's directory is unavailable."}
      </p>
    {:else}
      <div class="mt-4 flex flex-wrap gap-x-4 gap-y-1 text-xs">
        {#each inspection.results as r}
          <span>
            {#if r.status === "detected"}
              <span class="text-green-600 dark:text-green-400">●</span> {r.kind}
            {:else if r.status === "not_detected"}
              <span class="text-gray-400">○</span>
              <span class="text-gray-500 dark:text-gray-400">{r.kind} — not detected</span>
            {:else}
              <span class="text-red-600 dark:text-red-400">▲</span>
              <span class="text-red-600 dark:text-red-400">{r.kind} — {r.error}</span>
            {/if}
          </span>
        {/each}
      </div>

      {#if detected.length > 0}
        <div
          role="tablist"
          class="mt-3 flex flex-wrap gap-1 border-b border-gray-200 dark:border-gray-700"
        >
          {#each detected as r}
            <button
              type="button"
              role="tab"
              aria-selected={activeKind === r.kind}
              onclick={() => (activeKind = r.kind)}
              class={`rounded-t-md px-3 py-1.5 text-sm font-medium ${
                activeKind === r.kind
                  ? "bg-gray-100 text-gray-900 dark:bg-gray-700 dark:text-gray-100"
                  : "text-gray-500 hover:text-gray-700 dark:text-gray-400"
              }`}
            >
              {trackerKind(r.tracker!)}
            </button>
          {/each}
        </div>

        {#each detected as r}
          {#if activeKind === r.kind}
            <div role="tabpanel" class="p-3">
              <div class="mb-2 flex justify-end">
                <button
                  type="button"
                  class="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-200"
                  onclick={() => load(r.kind)}
                >
                  re-detect
                </button>
              </div>
              <TrackerPanel tracker={r.tracker!} onerror={(m) => (banner = m)} />
            </div>
          {/if}
        {/each}
      {/if}
    {/if}
  {/if}
</main>

{#if editing && inspection}
  <div
    class="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 p-4"
    role="presentation"
    onclick={() => (editing = false)}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- click only stops the backdrop's close-on-click-outside; Escape is
         handled globally by <svelte:window>, so there's no keyboard pair -->
    <div
      class="w-11/12 max-w-lg"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
    >
      <EditProjectForm
        project={inspection.project}
        onSaved={handleSaved}
        onCancel={() => (editing = false)}
        onerror={(m) => (banner = m)}
      />
    </div>
  </div>
{/if}
