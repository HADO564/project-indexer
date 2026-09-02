<script lang="ts">
  let { message }: { message: string } = $props();
</script>

{#if message}
  <p
    class="mb-4 rounded-sm border border-rust/50 bg-panel px-3 py-2 text-sm text-rust"
  >
    {message}
  </p>
{/if}
