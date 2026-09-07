<script lang="ts">
  import { isPropertyQuery } from "$lib/views";
  import type { ViewMode } from "$lib/viewState";
  import BundledIcon from "./BundledIcon.svelte";
  import { iconMd, inputClass } from "./styles";

  let {
    mode = $bindable<ViewMode>("list"),
    query = $bindable(""),
    // Property names in use across the fetched projects. The `name: value`
    // syntax is only discoverable if the app says which names exist.
    knownPropertyKeys = [],
  }: {
    mode?: ViewMode;
    query?: string;
    knownPropertyKeys?: string[];
  } = $props();

  const modes: { value: ViewMode; icon: string; label: string }[] = [
    { value: "list", icon: "layers", label: "List" },
    { value: "grid", icon: "box", label: "Grid" },
  ];

  let input = $state<HTMLInputElement | null>(null);
  let focused = $state(false);

  // Ctrl+; (Cmd+; on macOS). Chosen because it is unclaimed: not a browser
  // shortcut, not a Windows or macOS system binding, and not one of the
  // near-universal app keys — Ctrl+F find, Ctrl+K command palette, Ctrl+L
  // address bar, Ctrl+E, Ctrl+P — that an app should leave alone.
  //
  // A window keydown listener, deliberately *not* the global-shortcut plugin:
  // that registers system-wide and would take the key from every other
  // application, which is the wrong scope for focusing a text box.
  function handleKeydown(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key === ";") {
      event.preventDefault();
      input?.focus();
      input?.select();
      return;
    }
    if (event.key === "Escape" && document.activeElement === input) {
      // Clear first, blur only if already empty — Escape on a full box far
      // more often means "undo this search" than "leave the box".
      if (query.length > 0) query = "";
      else input?.blur();
    }
  }

  // Show the tip while the box has focus, and keep it up once the query looks
  // like a property query, so the syntax is confirmed as it is typed.
  const showTip = $derived(focused || isPropertyQuery(query));
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex flex-col gap-1.5">
  <div class="flex items-center gap-2">
    <div class="relative min-w-0 flex-1">
      <input
        bind:this={input}
        bind:value={query}
        onfocus={() => (focused = true)}
        onblur={() => (focused = false)}
        type="search"
        placeholder="Search name, path, tag or property"
        aria-label="Search projects"
        aria-describedby="search-tip"
        class={`h-10 w-full pr-16 ${inputClass}`}
      />
      <kbd
        class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 rounded-sm border border-line px-1.5 py-0.5 font-display text-[12px] text-phos-faint"
        aria-hidden="true"
      >
        Ctrl+;
      </kbd>
    </div>

    <div class="flex shrink-0 items-center gap-0.5 rounded-sm border border-line bg-panel-2 p-0.5">
      {#each modes as m}
        <button
          type="button"
          onclick={() => (mode = m.value)}
          aria-pressed={mode === m.value}
          title={m.label}
          aria-label={`${m.label} view`}
          class={`inline-flex h-9 w-9 items-center justify-center rounded-sm ${
            mode === m.value ? "bg-panel text-accent" : "text-phos-dim hover:text-phos"
          }`}
        >
          <BundledIcon name={m.icon} class={iconMd} />
        </button>
      {/each}
    </div>
  </div>

  {#if showTip}
    <p id="search-tip" class="text-[12px] text-phos-faint">
      Search a property with <span class="font-mono text-phos-dim">name: value</span> — e.g.
      <span class="font-mono text-phos-dim">client: acme</span>. A bare
      <span class="font-mono text-phos-dim">name:</span> finds every project that has it.
      {#if knownPropertyKeys.length > 0}
        <span>In use:</span>
        {#each knownPropertyKeys as key, i}<button
            type="button"
            class="font-mono text-accent hover:text-glow"
            onclick={() => {
              query = `${key}: `;
              input?.focus();
            }}>{key}:</button
          >{i < knownPropertyKeys.length - 1 ? ", " : ""}{/each}
      {/if}
    </p>
  {/if}
</div>
