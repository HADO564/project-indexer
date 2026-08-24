<script lang="ts">
  import { deleteProjectDirectory } from "$lib/api/projects";
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

  // Unchecked by default: deleting always removes the directory, but keeps
  // the tracked metadata around (soft-deleted, recoverable from the bin)
  // unless the user opts into the permanent purge.
  let permanentlyDeleteMetadata = $state(false);
  let deleting = $state(false);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onCancel();
  }

  async function handleConfirm() {
    deleting = true;
    try {
      await deleteProjectDirectory(project.id, permanentlyDeleteMetadata);
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
      This deletes <code class="break-all">{project.directory}</code> from disk (cannot be undone).
    </p>
    <label class="mt-3 mb-4 flex items-start gap-2 text-sm text-red-700 dark:text-red-400">
      <input type="checkbox" bind:checked={permanentlyDeleteMetadata} class="mt-0.5" />
      <span>
        Also permanently forget this project, instead of keeping it in the bin
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
