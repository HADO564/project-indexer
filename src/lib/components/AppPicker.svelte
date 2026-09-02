<script lang="ts">
  import { listInstalledApps } from "$lib/api/apps";
  import type { InstalledApp } from "$lib/api/types";
  import { inputClass, labelClass } from "./styles";

  let {
    value = $bindable(""),
    onerror,
  }: {
    value?: string;
    onerror?: (message: string) => void;
  } = $props();

  let installedApps = $state<InstalledApp[]>([]);
  let installedAppsLoaded = false;
  let pickerOpen = $state(false);

  // Subsequence match so "vscode" finds "Visual Studio Code".
  function fuzzyMatch(query: string, target: string): boolean {
    const q = query.toLowerCase().replace(/\s+/g, "");
    const t = target.toLowerCase().replace(/\s+/g, "");
    let qi = 0;
    for (let ti = 0; ti < t.length && qi < q.length; ti++) {
      if (t[ti] === q[qi]) qi++;
    }
    return qi === q.length;
  }

  let filteredApps = $derived(
    value.trim().length === 0
      ? []
      : installedApps.filter((app) => fuzzyMatch(value, app.name)).slice(0, 8),
  );

  async function ensureAppsLoaded() {
    pickerOpen = true;
    if (installedAppsLoaded) return;
    try {
      installedApps = await listInstalledApps();
      installedAppsLoaded = true;
    } catch (err) {
      onerror?.((err as Error).message);
    }
  }

  function selectApp(app: InstalledApp) {
    value = app.path;
    pickerOpen = false;
  }

  function closeSoon() {
    setTimeout(() => (pickerOpen = false), 150);
  }
</script>

<label class={`relative ${labelClass}`}>
  Open with
  <input
    bind:value
    placeholder="Search installed apps, e.g. vscode"
    autocomplete="off"
    onfocus={ensureAppsLoaded}
    onblur={closeSoon}
    class={inputClass}
  />
  {#if pickerOpen && filteredApps.length > 0}
    <ul
      class="absolute top-full right-0 left-0 z-10 mt-1 max-h-56 overflow-y-auto rounded-sm border border-line bg-panel p-1"
    >
      {#each filteredApps as app}
        <li>
          <button
            type="button"
            onclick={() => selectApp(app)}
            class="flex w-full flex-col gap-0.5 rounded-sm px-2 py-1.5 text-left hover:bg-panel-2"
          >
            <span class="text-sm text-phos">{app.name}</span>
            <span class="text-xs text-phos-dim">{app.path}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</label>
