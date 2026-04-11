<template>
  <router-view v-slot="{ Component }">
    <keep-alive :include="['GalleryLayout']">
      <component :is="Component" />
    </keep-alive>
  </router-view>
</template>
