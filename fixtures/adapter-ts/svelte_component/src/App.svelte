<script lang="ts">
  import { onMount } from "svelte";
  import { UserStore } from "./stores/UserStore";

  let user = UserStore.get();

  onMount(() => {
    UserStore.load();
  });
</script>

<main>
  <h1>{user.name}</h1>
</main>
