<script lang="ts">
  import { importScanned, listDetectorKinds, scanFolder } from "$lib/api/scan";
  import type { Candidate, ImportReport, ScanReport } from "$lib/api/types";
  import {
    loadScanSettings,
    saveScanSettings,
    toScanRequest,
    type ScanSettings,
  } from "$lib/scanSettings";
  import DirectoryField from "./DirectoryField.svelte";
  import { buttonClass, inputClass, labelClass, primaryButtonClass } from "./styles";

  let {
    onImported,
    onClose,
    onerror,
  }: {
    onImported: (report: ImportReport) => void;
    onClose: () => void;
    onerror: (message: string) => void;
  } = $props();

  // configure → review → result. One modal rather than three, because the
  // review step only makes sense as the middle of this flow.
  let step = $state<"configure" | "review" | "result">("configure");
  let kinds = $state<string[]>([]);
  let settings = $state<ScanSettings>(loadScanSettings([]));
  let scanning = $state(false);
  let importing = $state(false);
  let report = $state<ScanReport | null>(null);
  let result = $state<ImportReport | null>(null);
  let selected = $state<Set<string>>(new Set());
  let names = $state<Record<string, string>>({});
  let showTracked = $state(false);

  // The tick-list is built from what the binary registers, never a hardcoded
  // array — invariant 1, "a new detector is implement + register, zero
  // frontend code".
  $effect(() => {
    listDetectorKinds()
      .then((available) => {
        kinds = available;
        settings = loadScanSettings(available);
      })
      .catch((err: Error) => onerror(err.message));
  });

  const untracked = $derived(report?.candidates.filter((c) => !c.already_tracked) ?? []);
  const tracked = $derived(report?.candidates.filter((c) => c.already_tracked) ?? []);
  const visible = $derived(showTracked ? (report?.candidates ?? []) : untracked);

  function toggleDetector(kind: string) {
    settings.detectors = settings.detectors.includes(kind)
      ? settings.detectors.filter((k) => k !== kind)
      : [...settings.detectors, kind];
  }

  async function runScan() {
    scanning = true;
    try {
      const found = await scanFolder(toScanRequest(settings));
      report = found;
      // Already-tracked rows start unticked: importing them is a no-op, and
      // pre-selecting them would make the count lie about what will happen.
      selected = new Set(found.candidates.filter((c) => !c.already_tracked).map((c) => c.directory));
      names = Object.fromEntries(found.candidates.map((c) => [c.directory, c.suggested_name]));
      step = "review";
    } catch (err) {
      onerror((err as Error).message);
    } finally {
      scanning = false;
    }
  }

  async function runImport() {
    importing = true;
    try {
      const selections = [...selected].map((directory) => ({
        directory,
        name: names[directory] ?? "",
      }));
      const imported = await importScanned(selections);
      result = imported;
      // Written on commit, not on scan: an abandoned review leaves nothing
      // behind, so an exploratory scan cannot overwrite settings that worked.
      saveScanSettings(settings);
      step = "result";
      onImported(imported);
    } catch (err) {
      onerror((err as Error).message);
    } finally {
      importing = false;
    }
  }

  function toggleRow(directory: string) {
    const next = new Set(selected);
    if (next.has(directory)) next.delete(directory);
    else next.add(directory);
    selected = next;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="fixed inset-0 z-[100] flex items-center justify-center bg-void/85"
  role="presentation"
  onclick={onClose}
  onkeydown={handleKeydown}
>
  <div
    class="max-h-[85vh] w-11/12 max-w-3xl overflow-y-auto rounded-sm border border-line bg-panel p-6"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="scan-folder-title"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="mb-4 flex items-center justify-between">
      <h2 id="scan-folder-title" class="mt-0 text-lg font-semibold text-phos">
        <span class="text-gold">//</span> scan a folder for projects
      </h2>
      <button type="button" onclick={onClose} class={buttonClass}>Close</button>
    </div>

    {#if step === "configure"}
      <div class="flex flex-col gap-4">
        <DirectoryField
          bind:value={settings.scanRoot}
          label="Folder to scan"
          required
          {onerror}
        />

        <fieldset class="flex flex-col gap-2">
          <legend class={labelClass}>How deep</legend>
          <label class="flex items-center gap-2 text-sm text-phos-dim">
            <input type="radio" name="scan-mode" bind:group={settings.mode} value="quick" />
            Quick — only the folders directly inside
          </label>
          <label class="flex items-center gap-2 text-sm text-phos-dim">
            <input type="radio" name="scan-mode" bind:group={settings.mode} value="deep" />
            Deep — descend
            <input
              type="number"
              min="1"
              max="10"
              bind:value={settings.depth}
              disabled={settings.mode !== "deep"}
              class="{inputClass} w-16"
            />
            levels
          </label>
        </fieldset>

        <fieldset class="flex flex-col gap-2">
          <legend class={labelClass}>Scan for</legend>
          {#each kinds as kind (kind)}
            <label class="flex items-center gap-2 text-sm text-phos-dim">
              <input
                type="checkbox"
                checked={settings.detectors.includes(kind)}
                onchange={() => toggleDetector(kind)}
              />
              {kind}
            </label>
          {/each}
          <p class="text-sm text-phos-dim">
            This decides which folders are worth importing. Whatever you import is
            checked against every detector, so a folder that is both lands once
            carrying both.
          </p>
        </fieldset>

        <label class="flex items-center gap-2 text-sm text-phos-dim">
          <input type="checkbox" bind:checked={settings.includeIgnored} />
          Include node_modules, target, build and other usually-ignored folders
        </label>

        <div class="flex justify-end gap-2">
          <button type="button" class={buttonClass} onclick={onClose}>Cancel</button>
          <button
            type="button"
            class={primaryButtonClass}
            disabled={scanning || !settings.scanRoot || settings.detectors.length === 0}
            onclick={runScan}
          >
            {scanning ? "Scanning…" : "Scan"}
          </button>
        </div>
      </div>
    {:else if step === "review"}
      <div class="flex flex-col gap-3">
        <p class="text-sm text-phos-dim">
          Found {untracked.length} new
          {untracked.length === 1 ? "project" : "projects"} in {report?.visited ?? 0} folders.
        </p>

        {#if report?.stopped_early}
          <p class="rounded-sm border border-amber/50 bg-panel px-3 py-2 text-sm text-amber">
            Stopped after 50,000 folders — these results are incomplete. Narrow the
            folder or reduce the depth.
          </p>
        {/if}

        {#if tracked.length > 0}
          <button
            type="button"
            class="self-start text-sm text-phos-dim underline"
            onclick={() => (showTracked = !showTracked)}
          >
            {showTracked ? "Hide" : "Show"} {tracked.length} already tracked
          </button>
        {/if}

        <div class="flex gap-2">
          <button
            type="button"
            class={buttonClass}
            onclick={() => (selected = new Set(untracked.map((c) => c.directory)))}
          >
            Select all
          </button>
          <button type="button" class={buttonClass} onclick={() => (selected = new Set())}>
            Select none
          </button>
        </div>

        <ul class="flex flex-col gap-2">
          {#each visible as candidate (candidate.directory)}
            <li class="flex items-center gap-2 border border-line p-2">
              <input
                type="checkbox"
                checked={selected.has(candidate.directory)}
                disabled={candidate.already_tracked}
                onchange={() => toggleRow(candidate.directory)}
              />
              <input
                class="{inputClass} w-56"
                bind:value={names[candidate.directory]}
                disabled={candidate.already_tracked}
              />
              <span class="flex-1 truncate text-sm text-phos-dim" title={candidate.directory}>
                {candidate.directory}
              </span>
              <span class="text-xs text-phos-dim">{candidate.matched_kinds.join(", ")}</span>
              {#if candidate.already_tracked}
                <span class="text-xs text-phos-dim">already tracked</span>
              {:else if names[candidate.directory] !== candidate.suggested_name}
                <span class="text-xs text-phos-dim">renamed</span>
              {/if}
            </li>
          {/each}
        </ul>

        <div class="flex justify-end gap-2">
          <button type="button" class={buttonClass} onclick={() => (step = "configure")}>
            Back
          </button>
          <button
            type="button"
            class={primaryButtonClass}
            disabled={importing || selected.size === 0}
            onclick={runImport}
          >
            {importing ? "Importing…" : `Import ${selected.size}`}
          </button>
        </div>
      </div>
    {:else}
      <div class="flex flex-col gap-3">
        <p class="text-phos">
          Imported {result?.imported.length ?? 0}
          {(result?.imported.length ?? 0) === 1 ? "project" : "projects"}.
          {#if (result?.skipped ?? 0) > 0}
            Skipped {result?.skipped} already tracked.
          {/if}
        </p>

        {#if (result?.failures.length ?? 0) > 0}
          <div>
            <p class="mb-1 text-phos">{result?.failures.length} could not be imported:</p>
            <ul class="flex flex-col gap-1 text-sm text-phos-dim">
              {#each result?.failures ?? [] as failure (failure.directory)}
                <li>{failure.directory} — {failure.message}</li>
              {/each}
            </ul>
          </div>
        {/if}

        <div class="flex justify-end">
          <button type="button" class={primaryButtonClass} onclick={onClose}>Done</button>
        </div>
      </div>
    {/if}
  </div>
</div>
