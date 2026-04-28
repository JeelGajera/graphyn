<template>
  <div>{{ user.name }}</div>
</template>

<script setup lang="ts">
import type { User } from "../types/user";

defineProps<{ user: User }>();
</script>
