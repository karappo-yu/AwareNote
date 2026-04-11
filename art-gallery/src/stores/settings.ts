import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getSettings, updateSettings, type UserDataItem } from '../api'

export const useSettings = defineStore('settings', () => {
  // 全局设置缓存（key → value）
  const settingsMap = ref<Record<string, string>>({})
  const loaded = ref(false)

  /** 启动时从后端拉取所有全局设置 */
  async function fetchSettings() {
    try {
      const items = await getSettings()
      const map: Record<string, string> = {}
      for (const item of items) {
        map[item.key] = item.value
      }
      settingsMap.value = map
      loaded.value = true
    } catch {
      console.warn('Failed to fetch settings from backend')
    }
  }

  /** 获取设置值，不存在返回默认值 */
  function get(key: string, defaultValue?: string): string | undefined {
    return settingsMap.value[key] ?? defaultValue
  }

  /** 获取设置值（数字类型） */
  function getNumber(key: string, defaultValue: number): number {
    const val = settingsMap.value[key]
    if (val === undefined) return defaultValue
    const num = Number(val)
    return isNaN(num) ? defaultValue : num
  }

  /** 设置单个值 — 同时写后端 + 更新本地缓存 */
  async function set(key: string, value: string) {
    settingsMap.value[key] = value
    try {
      await updateSettings([{ key, value }])
    } catch {
      console.warn(`Failed to persist setting "${key}"`)
    }
  }

  /** 批量设置 */
  async function setBatch(items: UserDataItem[]) {
    for (const item of items) {
      settingsMap.value[item.key] = item.value
    }
    try {
      await updateSettings(items)
    } catch {
      console.warn('Failed to persist batch settings')
    }
  }

  return {
    settingsMap,
    loaded,
    fetchSettings,
    get,
    getNumber,
    set,
    setBatch,
  }
})
