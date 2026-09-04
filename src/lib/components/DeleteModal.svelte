<script lang="ts">
  import { deleteProjectDirectory, untrackProject } from "$lib/api/projects";
  import type { Project } from "$lib/api/types";
  import { buttonClass, dangerButtonClass, primaryButtonClass } from "./styles";

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

  type Mode = "disk" | "untrack";
  // Defaults to the non-destructive mode: opening this dialog and hitting
  // Delete removes the entry and leaves the folder on disk. Deleting the
  // directory is a deliberate second choice, never the one already selected.
  let mode = $state<Mode>("untrack");

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
      if (mode === "untrack") {
        await untrackProject(project.id);
      } else {
        await deleteProjectDirectory(project.id, permanentlyDeleteMetadata);
      }
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
  class="fixed inset-0 z-[100] flex items-center justify-center bg-void/85"
  role="presentation"
  onclick={onCancel}
  onkeydown={handleKeydown}
>
  <div
    class="w-11/12 max-w-md rounded-sm border border-line bg-panel p-6"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="delete-modal-title"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <h2
      id="delete-modal-title"
      class="mt-0 text-lg font-semibold text-phos"
    >
      Delete "{project.name}"?
    </h2>

    <div class="mt-3 flex flex-col gap-2">
      <label class="flex items-start gap-2 text-sm text-phos-dim">
        <input
          type="radio"
          name="delete-mode"
          value="untrack"
          bind:group={mode}
          class="mt-0.5"
        />
        <span>
          Just remove it from this app — keep the folder on disk untouched
        </span>
      </label>
      <label class="flex items-start gap-2 text-sm text-phos-dim">
        <input
          type="radio"
          name="delete-mode"
          value="disk"
          bind:group={mode}
          class="mt-0.5"
        />
        <span>
          Delete <code class="break-all">{project.directory}</code> from disk (cannot be undone)
        </span>
      </label>
    </div>

    {#if mode === "disk"}
      <label class="mt-3 mb-4 flex items-start gap-2 text-sm text-rust">
        <input type="checkbox" bind:checked={permanentlyDeleteMetadata} class="mt-0.5" />
        <span>
          Also permanently forget this project, instead of keeping it in the bin
        </span>
      </label>
    {:else}
      <p class="mt-3 mb-4 text-sm text-phos-dim">
        The directory itself is left alone. You can re-add it later by creating a project
        pointed at the same folder.
      </p>
    {/if}

    <div class="flex gap-2">
      <!-- The button says what it will actually do, and only wears the
           destructive styling when the destructive mode is selected. -->
      <button
        type="button"
        onclick={handleConfirm}
        disabled={deleting}
        class={mode === "disk" ? dangerButtonClass : primaryButtonClass}
      >
        {#if deleting}
          {mode === "disk" ? "Deleting…" : "Removing…"}
        {:else}
          {mode === "disk" ? "Delete from disk" : "Remove"}
        {/if}
      </button>
      <button type="button" onclick={onCancel} disabled={deleting} class={buttonClass}>
        Cancel
      </button>
    </div>
  </div>
</div>
