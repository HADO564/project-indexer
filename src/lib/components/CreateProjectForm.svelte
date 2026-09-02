<script lang="ts">
  import { createProject, detectProjectTrackers } from "$lib/api/projects";
  import type { CreateProjectInput, GitInfo, Tracker } from "$lib/api/types";
  import DirectoryField from "./DirectoryField.svelte";
  import { cardClass, inputClass, labelClass, primaryButtonClass } from "./styles";

  let {
    onCreated,
    onerror,
  }: {
    onCreated: () => void | Promise<void>;
    onerror?: (message: string) => void;
  } = $props();

  let name = $state("");
  let directory = $state("");
  let description = $state("");
  let tags = $state("");
  let creating = $state(false);

  function parseTags(raw: string): string[] {
    return raw
      .split(",")
      .map((t) => t.trim())
      .filter((t) => t.length > 0);
  }

  function isGitTracker(tracker: Tracker): tracker is { Git: GitInfo } {
    return typeof tracker !== "string" && "Git" in tracker;
  }

  // "https://github.com/user/my-repo.git" / "git@github.com:user/my-repo.git" -> "my-repo"
  function repoNameFromUrl(url: string): string | null {
    const withoutTrailingSlash = url.trim().replace(/\/+$/, "");
    const withoutGitSuffix = withoutTrailingSlash.endsWith(".git")
      ? withoutTrailingSlash.slice(0, -4)
      : withoutTrailingSlash;
    return withoutGitSuffix.split(/[/:]/).filter(Boolean).pop() ?? null;
  }

  // "D:\Projects\Friction\" / "/home/user/friction/" -> "friction"
  function folderNameFromDirectory(directory: string): string | null {
    const withoutTrailingSlash = directory.trim().replace(/[\\/]+$/, "");
    return withoutTrailingSlash.split(/[\\/]/).filter(Boolean).pop() ?? null;
  }

  // Prefers the git remote's repo name (what "using the gitector feature"
  // means here) and falls back to the folder's own name for anything else
  // (not a repo, or a repo with no remote configured).
  function suggestProjectName(trackers: Tracker[], directory: string): string | null {
    const repoUrl = trackers.find(isGitTracker)?.Git.repo_url;
    return (repoUrl && repoNameFromUrl(repoUrl)) || folderNameFromDirectory(directory);
  }

  // Best-effort: runs detection against the picked directory to suggest a
  // name. Never overwrites a name the user already typed, and a detection
  // failure just means no suggestion — not worth surfacing as an error.
  async function handleDirectoryPicked(dir: string) {
    if (name.trim().length > 0) return;
    try {
      const trackers = await detectProjectTrackers(dir);
      if (name.trim().length > 0) return;
      const suggested = suggestProjectName(trackers, dir);
      if (suggested) name = suggested;
    } catch {
      // No suggestion — the user can still type a name manually.
    }
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();
    creating = true;
    try {
      const input: CreateProjectInput = {
        name,
        directory,
        description: description || null,
        tags: parseTags(tags),
      };
      await createProject(input);
      name = "";
      directory = "";
      description = "";
      tags = "";
      await onCreated();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      creating = false;
    }
  }
</script>

<section class={`mb-6 ${cardClass}`}>
  <h2 class="mb-3 font-display text-[14px] uppercase tracking-wide text-phos-dim"><span class="text-gold">//</span> new project</h2>
  <form onsubmit={handleSubmit} class="flex flex-col gap-3">
    <label class={labelClass}>
      Name
      <input bind:value={name} required placeholder="My project" class={inputClass} />
    </label>
    <DirectoryField
      bind:value={directory}
      required
      onerror={(m) => onerror?.(m)}
      onPicked={handleDirectoryPicked}
    />
    <label class={labelClass}>
      Description
      <input bind:value={description} placeholder="Optional description" class={inputClass} />
    </label>
    <label class={labelClass}>
      Tags (comma separated)
      <input bind:value={tags} placeholder="rust, tauri" class={inputClass} />
    </label>
    <button type="submit" disabled={creating} class={`self-start ${primaryButtonClass}`}>
      {creating ? "Creating…" : "Create project"}
    </button>
  </form>
</section>
