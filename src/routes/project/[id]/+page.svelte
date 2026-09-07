<script lang="ts">
  import { page } from "$app/stores";
  import { listGroups } from "$lib/api/groups";
  import { listCustomIcons } from "$lib/api/icons";
  import { isOpenWithAppMissing, openProjectDirectory } from "$lib/api/opener";
  import { inspectProject, refreshProjectTrackers } from "$lib/api/projects";
  import type { Group, ProjectInspection } from "$lib/api/types";
  import { customIconSrc } from "$lib/icons";
  import EditProjectModal from "$lib/components/EditProjectModal.svelte";
  import ErrorBanner from "$lib/components/ErrorBanner.svelte";
  import ProjectIdentity from "$lib/components/ProjectIdentity.svelte";
  import TrackerPanel from "$lib/components/TrackerPanel.svelte";
  import { buttonClass } from "$lib/components/styles";
  import { trackerColor, trackerKind } from "$lib/trackers";

  // Route always supplies `id`; `?? ""` only satisfies the `string | undefined`
  // param type. `load()` bails on an empty id.
  let id = $derived($page.params.id ?? "");

  let inspection = $state<ProjectInspection | null>(null);
  let loadError = $state("");
  let banner = $state("");
  let loading = $state(false);
  let editing = $state(false);
  let activeKind = $state<string | null>(null);
  // The edit form offers a group and an icon here too, so this route needs the
  // same two lists the main page fetches. Both are best-effort: failing to
  // load them must not stop the detail view rendering.
  let groups = $state<Group[]>([]);
  let customIcons = $state<Map<string, string>>(new Map());

  async function loadGroups() {
    try {
      groups = await listGroups();
    } catch {
      groups = [];
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

  $effect(() => {
    loadGroups();
    loadCustomIcons();
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


<main class="mx-auto max-w-3xl px-4 py-8">
  <a href="/" class="font-display text-[14px] uppercase tracking-wide text-accent hover:underline">&larr; all projects</a>

  {#if loadError}
    <div class="mt-4">
      <ErrorBanner message={loadError} />
    </div>
  {:else if inspection}
    <div class="mt-4 flex items-start justify-between gap-4">
      <h1 class="text-xl text-phos">
        <span class="text-accent">&gt;</span>&nbsp;{inspection.project.name}
      </h1>
      <div class="flex shrink-0 gap-2">
        <button type="button" class={buttonClass} onclick={handleOpen}>Open</button>
        <button type="button" class={buttonClass} onclick={() => (editing = true)}>Edit</button>
        <button type="button" class={buttonClass} onclick={handleRefresh} disabled={loading}>
          {loading ? "···" : "Refresh"}
        </button>
      </div>
    </div>

    <div class="mt-3"><ErrorBanner message={banner} /></div>

    <div class="mt-2 rounded-sm border border-line bg-panel p-4">
      <ProjectIdentity project={inspection.project} />
    </div>

    {#if !inspection.directory_status.ok}
      <p class="mt-4 rounded-sm border border-amber/40 bg-panel p-3 text-sm text-amber">
        {inspection.directory_status.message ?? "This project's directory is unavailable."}
      </p>
    {:else}
      {@const shown = inspection.results.filter((r) => r.status !== "not_detected")}
      {@const missing = inspection.results.filter((r) => r.status === "not_detected")}

      {#if shown.length > 0}
        <div class="mt-4 flex flex-wrap gap-x-4 gap-y-1 font-display text-[14px]">
          {#each shown as r}
            {#if r.status === "detected"}
              <span style="color: {trackerColor(r.kind)}">● {r.kind}</span>
            {:else}
              <span class="text-rust">▲ {r.kind} — {r.error}</span>
            {/if}
          {/each}
        </div>
      {/if}

      {#if detected.length > 0}
        <div role="tablist" class="mt-3 flex flex-wrap gap-1 border-b border-line">
          {#each detected as r}
            {@const active = activeKind === r.kind}
            <button
              type="button"
              role="tab"
              aria-selected={active}
              onclick={() => (activeKind = r.kind)}
              class="-mb-px border-b-2 px-3 py-1.5 font-display text-[14px] transition-colors"
              style={active
                ? `color: ${trackerColor(r.kind)}; border-color: ${trackerColor(r.kind)}`
                : "border-color: transparent"}
              class:text-phos-dim={!active}
              class:hover:text-phos={!active}
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
                  class="font-display text-[13px] uppercase tracking-wide text-phos-faint hover:text-phos"
                  onclick={() => load(r.kind)}
                >
                  re-detect
                </button>
              </div>
              <TrackerPanel tracker={r.tracker!} onerror={(m) => (banner = m)} />
            </div>
          {/if}
        {/each}
      {:else if shown.length === 0}
        <p class="mt-4 text-sm text-phos-dim">
          No project type detected. Try Refresh, or check the directory still exists.
        </p>
      {/if}

      {#if missing.length > 0}
        <details class="mt-6 text-[11px]">
          <summary
            class="cursor-pointer font-display text-[13px] uppercase tracking-wide text-phos-faint hover:text-phos-dim"
          >
            Not detected ({missing.length})
          </summary>
          <div class="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 pl-3 text-phos-faint">
            {#each missing as r}<span>{r.kind}</span>{/each}
          </div>
        </details>
      {/if}
    {/if}
  {/if}
</main>

{#if editing && inspection}
  <EditProjectModal
    project={inspection.project}
    {groups}
    knownPropertyKeys={Object.keys(inspection.project.properties)}
    {customIcons}
    onIconsChanged={loadCustomIcons}
    onGroupsStale={loadGroups}
    onSaved={handleSaved}
    onClose={() => (editing = false)}
    onerror={(m) => (banner = m)}
  />
{/if}
