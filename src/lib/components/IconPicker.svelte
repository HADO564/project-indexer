<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { deleteCustomIcon, importCustomIcon } from "$lib/api/icons";
  import { ICON_NAMES } from "$lib/icons";
  import BundledIcon from "./BundledIcon.svelte";
  import { buttonClass, labelClass } from "./styles";

  let {
    value = $bindable<string | null>(null),
    customIcons,
    onIconsChanged,
    // Group icons come from the bundled set only: a custom SVG renders as an
    // <img> and cannot be tinted, and a sidebar entry must take its group's
    // colour.
    bundledOnly = false,
    onerror,
  }: {
    value?: string | null;
    customIcons: Map<string, string>;
    onIconsChanged?: () => void | Promise<void>;
    bundledOnly?: boolean;
    onerror?: (message: string) => void;
  } = $props();

  let importing = $state(false);

  const cell = "inline-flex h-8 w-8 items-center justify-center rounded-sm border-2";

  async function handleImport() {
    importing = true;
    try {
      const path = await open({
        multiple: false,
        filters: [{ name: "SVG", extensions: ["svg"] }],
      });
      if (typeof path !== "string") return;
      const icon = await importCustomIcon(path);
      await onIconsChanged?.();
      value = `custom:${icon.name}`;
    } catch (err) {
      // Import fails for content reasons as often as IO ones — an undefined
      // entity, a bare &, &nbsp; (an HTML entity XML does not define, common
      // in HTML-flavoured exports), nothing drawable left after sanitizing,
      // over 256 KB, a backslash in an attribute value. The message names the
      // reason and every one of them is something the user can fix, so
      // surface it rather than a generic failure.
      onerror?.((err as Error).message);
    } finally {
      importing = false;
    }
  }

  async function handleDeleteCustom(name: string) {
    try {
      await deleteCustomIcon(name);
      if (value === `custom:${name}`) value = null;
      await onIconsChanged?.();
    } catch (err) {
      onerror?.((err as Error).message);
    }
  }
</script>

<div class={labelClass}>
  Icon
  <div class="flex flex-wrap items-center gap-1">
    <button
      type="button"
      onclick={() => (value = null)}
      title="No icon"
      aria-label="No icon"
      aria-pressed={value === null}
      class={`${cell} text-phos-faint ${value === null ? "border-phos" : "border-line"}`}
    >
      —
    </button>
    {#each ICON_NAMES as name}
      <button
        type="button"
        onclick={() => (value = name)}
        title={name}
        aria-label={name}
        aria-pressed={value === name}
        class={`${cell} ${
          value === name ? "border-phos text-phos" : "border-transparent text-phos-dim hover:text-phos"
        }`}
      >
        <BundledIcon {name} class="h-4 w-4" />
      </button>
    {/each}
  </div>

  {#if !bundledOnly}
    <div class="mt-2 flex flex-wrap items-center gap-1">
      {#each [...customIcons] as [name, src] (name)}
        <span class="relative inline-flex">
          <button
            type="button"
            onclick={() => (value = `custom:${name}`)}
            title={name}
            aria-label={name}
            aria-pressed={value === `custom:${name}`}
            class={`${cell} ${value === `custom:${name}` ? "border-phos" : "border-transparent"}`}
          >
            <!-- A custom icon renders only through <img> with a data: URI —
                 inert regardless of what the sanitizer missed. Never {@html}. -->
            <img {src} alt="" class="h-4 w-4" />
          </button>
          <button
            type="button"
            onclick={() => handleDeleteCustom(name)}
            title={`Delete ${name}`}
            aria-label={`Delete icon ${name}`}
            class="absolute -top-1 -right-1 rounded-full bg-panel px-1 text-[10px] leading-none text-phos-faint hover:text-rust"
          >
            ×
          </button>
        </span>
      {/each}
      <button type="button" onclick={handleImport} disabled={importing} class={buttonClass}>
        {importing ? "Importing…" : "Import SVG…"}
      </button>
    </div>
  {/if}
</div>
