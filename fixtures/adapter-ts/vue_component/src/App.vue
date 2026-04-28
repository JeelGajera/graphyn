<template>
  <div class="app">
    <UserCard :user="currentUser" />
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { UserCard } from "./components/UserCard.vue";
import { UserService } from "./services/user-service";

const currentUser = ref(UserService.getCurrent());
</script>

<style scoped>
.app { padding: 1rem; }
</style>
