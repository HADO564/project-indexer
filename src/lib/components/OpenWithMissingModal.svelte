<script lang="ts">
  import { openProjectDirectory, openProjectInExplorer } from "$lib/api/opener";
  import { updateProject } from "$lib/api/projects";
  import type { Project } from "$lib/api/types";
  import AppPicker from "./AppPicker.svelte";
  import { buttonClass, primaryButtonClass } from "./styles";

  let {
    project,
    onOpened,
    onClose,
    onerror,
  }: {
    project: Project;
    onOpened: () => void | Promise<void>;
    onClose: () => void;
    onerror?: (message: string) => void;
  } = $props();

  // "Yes" swaps this modal's content for the app picker rather than opening
  // a second modal on top of it.
  let choosingApp = $state(false);
  let newOpenWith = $state("");
  let busy = $state(false);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onClose();
  }

  async function handleOpenInExplorer() {
    busy = true;
    try {
      await openProjectInExplorer(project.id);
      await onOpened();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      busy = false;
    }
  }

  function handleChooseAnotherApp() {
    choosingApp = true;
  }

  async function handleConfirmNewApp() {
    if (!newOpenWith.trim()) return;
    busy = true;
    try {
      await updateProject(project.id, { open_with: newOpenWith });
      await openProjectDirectory(project.id);
      await onOpened();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="fixed inset-0 z-[100] flex items-center justify-center bg-black/50"
  role="presentation"
  onclick={onClose}
  onkeydown={handleKeydown}
>
  <div
    class="w-11/12 max-w-md rounded-lg bg-white p-6 shadow-2xl dark:bg-gray-800"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="open-with-missing-title"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    {#if !choosingApp}
      <h2
        id="open-with-missing-title"
        class="mt-0 text-lg font-semibold text-gray-900 dark:text-gray-100"
      >
        App not found
      </h2>
      <p class="mt-2 text-sm text-gray-600 dark:text-gray-300">
        The app associated with this project has been removed or cannot be found. Would you like
        to open it with another app?
      </p>
      <div class="mt-4 flex flex-wrap gap-2">
        <button type="button" onclick={handleOpenInExplorer} disabled={busy} class={buttonClass}>
          Open in Explorer
        </button>
        <button
          type="button"
          onclick={handleChooseAnotherApp}
          disabled={busy}
          class={primaryButtonClass}
        >
          Yes
        </button>
        <button type="button" onclick={onClose} disabled={busy} class={buttonClass}>No</button>
      </div>
    {:else}
      <h2
        id="open-with-missing-title"
        class="mt-0 text-lg font-semibold text-gray-900 dark:text-gray-100"
      >
        Choose an app
      </h2>
      <div class="mt-3">
        <AppPicker bind:value={newOpenWith} onerror={(m) => onerror?.(m)} />
      </div>
      <div class="mt-4 flex gap-2">
        <button
          type="button"
          onclick={handleConfirmNewApp}
          disabled={busy || !newOpenWith.trim()}
          class={primaryButtonClass}
        >
          {busy ? "Opening…" : "Open"}
        </button>
        <button type="button" onclick={onClose} disabled={busy} class={buttonClass}>Cancel</button>
      </div>
    {/if}
  </div>
</div>
