<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { deleteCustomIcon, importCustomIcon } from "$lib/api/icons";
  import { customIconName, isCustomIcon, ICON_NAMES } from "$lib/icons";
  import BundledIcon from "./BundledIcon.svelte";
  import { buttonClass, iconMd, labelClass } from "./styles";

  let {
    value = $bindable<string | null>(null),
    customIcons,
    onIconsChanged,
    // Group icons come from the bundled set only when this is set: a custom
    // SVG renders as an <img> and cannot be tinted.
    bundledOnly = false,
    // A project may have no icon (it falls back to the folder glyph); a group
    // always has one, so it gets no "none" option to pick.
    allowNone = true,
    onerror,
  }: {
    value?: string | null;
    customIcons: Map<string, string>;
    onIconsChanged?: () => void | Promise<void>;
    bundledOnly?: boolean;
    allowNone?: boolean;
    onerror?: (message: string) => void;
  } = $props();

  let importing = $state(false);
  // Collapsed by default. Twenty-five glyphs plus the imported ones is a tall
  // block to carry permanently in a dialog that also holds a group select,
  // a colour row and the property editor — and the icon is picked once and
  // then left alone.
  let open_ = $state(false);

  const cell = "inline-flex h-10 w-10 items-center justify-center rounded-sm border-2";

  const customSrc = $derived(
    value && isCustomIcon(value) ? customIcons.get(customIconName(value)) : undefined,
  );

  function choose(next: string | null) {
    value = next;
    open_ = false;
  }

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
      // over 256 KB, or input that is not an SVG at all. The message names the
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

  <!-- The current choice stays visible while collapsed, so the summary is
       still an answer to "what is this project's icon?". -->
  <div class="flex items-center gap-2">
    <span class={`${cell} border-line text-phos`}>
      {#if customSrc}
        <!-- A custom icon renders only through <img> with a data: URI —
             inert regardless of what the sanitizer missed. Never {@html}. -->
        <img src={customSrc} alt="" class={iconMd} />
      {:else if value}
        <BundledIcon name={value} class={iconMd} />
      {:else}
        <span class="font-display text-[15px] text-phos-faint">—</span>
      {/if}
    </span>

    <span class="min-w-0 flex-1 truncate font-mono text-[13px] normal-case text-phos-dim">
      {value ?? "No icon"}
    </span>

    <button
      type="button"
      onclick={() => (open_ = !open_)}
      class={buttonClass}
      aria-expanded={open_}
    >
      {open_ ? "Done" : "Change"}
    </button>
  </div>

  {#if open_}
    <div class="mt-1 flex flex-wrap items-center gap-1 rounded-sm border border-line p-2">
      {#if allowNone}
        <button
          type="button"
          onclick={() => choose(null)}
          title="No icon"
          aria-label="No icon"
          aria-pressed={value === null}
          class={`${cell} text-phos-faint ${value === null ? "border-phos" : "border-line"}`}
        >
          —
        </button>
      {/if}
      {#each ICON_NAMES as name}
        <button
          type="button"
          onclick={() => choose(name)}
          title={name}
          aria-label={name}
          aria-pressed={value === name}
          class={`${cell} ${
            value === name
              ? "border-phos text-phos"
              : "border-transparent text-phos-dim hover:text-phos"
          }`}
        >
          <BundledIcon {name} class={iconMd} />
        </button>
      {/each}

      {#if !bundledOnly}
        <div class="mt-1 flex w-full flex-wrap items-center gap-1 border-t border-line pt-2">
          {#each [...customIcons] as [name, src] (name)}
            <span class="relative inline-flex">
              <button
                type="button"
                onclick={() => choose(`custom:${name}`)}
                title={name}
                aria-label={name}
                aria-pressed={value === `custom:${name}`}
                class={`${cell} ${
                  value === `custom:${name}` ? "border-phos" : "border-transparent"
                }`}
              >
                <img {src} alt="" class={iconMd} />
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
  {/if}
</div>
