import { createRouter, createWebHistory } from 'vue-router'

const routes = [
  {
    path: '/',
    component: () => import('../layouts/GalleryLayout.vue'),
    children: [
      { path: '', name: 'gallery', component: () => import('../pages/GalleryPage.vue') },
      { path: 'favorites', name: 'favorites', component: () => import('../pages/FavoritesPage.vue') },
      { path: 'settings', name: 'settings', component: () => import('../pages/SettingsPage.vue') }
    ]
  },
  {
    path: '/book/:id',
    name: 'detail',
    component: () => import('../pages/DetailPage.vue'),
    props: true
  }
]

export default createRouter({
  history: createWebHistory(),
  routes,
  scrollBehavior(to, from) {
    // 从详情页返回列表页时，不重置滚动位置（由 keep-alive + onActivated 恢复）
    if (from.name === 'detail' && (to.name === 'gallery' || to.name === 'favorites' || to.name === 'settings')) {
      return false
    }
    return { top: 0 }
  }
})
