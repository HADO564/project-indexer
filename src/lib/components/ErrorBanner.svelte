<script lang="ts">
  let { message }: { message: string } = $props();
</script>

{#if message}
  <p
    class="mb-4 rounded-md bg-red-100 px-3 py-2 text-sm text-red-800 dark:bg-red-950 dark:text-red-300"
  >
    {message}
  </p>
{/if}
