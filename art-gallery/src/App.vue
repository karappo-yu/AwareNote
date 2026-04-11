<template>
  <router-view v-slot="{ Component }">
    <keep-alive :include="['GalleryLayout']">
      <component :is="Component" />
    </keep-alive>
  </router-view>
</template>

<script setup lang="ts">
import { useSettings } from './stores/settings'

const settings = useSettings()
settings.fetchSettings()
</script>
