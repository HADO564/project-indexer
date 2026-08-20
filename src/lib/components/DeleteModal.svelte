<script lang="ts">
  import { deleteDirectory } from "$lib/api/filesystem";
  import { deleteProject } from "$lib/api/projects";
  import type { Project } from "$lib/api/types";
  import { buttonClass, dangerButtonClass } from "./styles";

  let {
    project,
    onDeleted,
    onCancel,
    onerror,
  }: {
    project: Project;
    onDeleted: () => void | Promise<void>;
    onCancel: () => void;
    onerror?: (message: string) => void;
  } = $props();

  let deleteDirectoryToo = $state(false);
  let deleting = $state(false);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onCancel();
  }

  async function handleConfirm() {
    deleting = true;
    try {
      if (deleteDirectoryToo) {
        await deleteDirectory(project.directory);
      }
      await deleteProject(project.id);
      await onDeleted();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      deleting = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="fixed inset-0 z-[100] flex items-center justify-center bg-black/50"
  role="presentation"
  onclick={onCancel}
  onkeydown={handleKeydown}
>
  <div
    class="w-11/12 max-w-md rounded-lg bg-white p-6 shadow-2xl dark:bg-gray-800"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="delete-modal-title"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <h2
      id="delete-modal-title"
      class="mt-0 text-lg font-semibold text-gray-900 dark:text-gray-100"
    >
      Delete "{project.name}"?
    </h2>
    <p class="mt-2 text-sm text-gray-600 dark:text-gray-300">
      This removes the project from Project Indexer.
    </p>
    <label class="mt-3 mb-4 flex items-start gap-2 text-sm text-red-700 dark:text-red-400">
      <input type="checkbox" bind:checked={deleteDirectoryToo} class="mt-0.5" />
      <span>
        Also delete <code class="break-all">{project.directory}</code> from disk (cannot be undone)
      </span>
    </label>
    <div class="flex gap-2">
      <button type="button" onclick={handleConfirm} disabled={deleting} class={dangerButtonClass}>
        {deleting ? "Deleting…" : "Delete"}
      </button>
      <button type="button" onclick={onCancel} disabled={deleting} class={buttonClass}>
        Cancel
      </button>
    </div>
  </div>
</div>
