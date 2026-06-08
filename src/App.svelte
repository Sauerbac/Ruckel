<!--
  App root. F1 mounts only the static shell (ADR-0019); data-bound components
  and IPC wiring arrive in later slices.
-->
<script lang="ts">
  import Shell from './lib/shell/Shell.svelte'
</script>

<Shell />
