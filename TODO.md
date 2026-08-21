# Art Gallery 开发 TODO

## 已完成

### 涂鸦模式（参考图标记）✅
- [x] `DoodleCanvas.vue` 画布组件：画笔（贝塞尔平滑）/ 橡皮擦（像素级擦除，destination-out 重放模型）/ 坐标归一化（窗口缩放时笔迹等比缩放）/ devicePixelRatio 适配
- [x] 画笔可涂出图片范围外：画布扩展至整个查看器主区域，笔迹坐标锚定图片矩形（越界坐标 <0/>1 合法），缩放窗口时笔迹仍钉在图片上
- [x] 参考图透明度调节（勾线辅助）：工具栏 5%~100% 滑块，低于 100% 时草稿向纯白衬底淡出；按书持久化（`doodle_image_opacity`）
- [x] 查看器底栏涂鸦开关 + 悬浮工具栏：6 色画笔、粗细滑块（画笔 1-12px / 橡皮擦 4-60px 独立记忆）、撤销/重做、清空（带确认）、完成；橡皮擦图标为内联 SVG（经典 Material Icons 字体无此字形）
- [x] 涂鸦模式下自动禁用翻页/缩放/拖拽/翻转/拼接/背景点击关闭，防止误触和画布错位
- [x] 笔画数据以 `doodle:<filename>` 为 key 存入 per-book user_data（`/api/books/:id/settings`），非侵入、零后端改动
- [x] 防抖自动保存（800ms），翻页/退出涂鸦/关闭查看器/组件卸载时立即落盘
- [x] 网格/瀑布流缩略图显示涂鸦角标（左上角铅笔图标）
- [x] 暂不支持：收藏虚拟书页、拼接页（按钮置灰并提示）

## 优先级 P0 — 近期必做

### 1. 收藏页跳转到原书
- [ ] 虚拟书查看器底栏加"跳转到原书"按钮
- [ ] 点击后路由到 `/books/:sourceBookId?page=N`
- [ ] DetailPage `onMounted` 读取 `?page=N` 参数，自动 `openViewer(N)`
- [ ] 瀑布流/查看器里 hover 显示来源书名（解决"忘了这页来自哪本书"的问题）

### 2. 已知 Bug 修复
- [ ] 取消收藏后加 Notify 提示 + undo 按钮（防误操作）
- [ ] `src/frontend/` 构建产物加入 `.gitignore`，减少 commit 噪音
- [ ] DetailPage 组件拆分（viewer / masonry / info-drawer 独立组件）

---

## 优先级 P1 — 核心功能扩展

### 3. CBZ 漫画格式支持
- [ ] 后端扫描时识别 `.cbz` 文件
- [ ] 用 Rust `zip` crate 解压到临时目录（或缓存目录）
- [ ] 按文件名排序提取图片列表，作为 `image_book` 处理
- [ ] 解析 ComicInfo.xml 元数据：
  - `Manga` 字段 → 自动设置 `reading_direction`（YesAndRightToLeft=rtl）
  - `Pages/Page[DoublePage]` → 自动生成 spread 记录
  - `Series`+`Number` / `Title` → 替代文件名作为书名
  - `Writer`/`Penciller` → 按作者筛选
- [ ] 无 ComicInfo.xml 时：默认 `reading_direction = 'rtl'`（漫画格式默认 RTL）
- [ ] 缓存策略：解压后缓存图片路径，避免每次重新解压
- [ ] 前端无感知，统一走 image_book 的渲染逻辑

### 4. 单页/Spread 导出（两种方式）
- [ ] 导出目录默认 `~/Pictures/ArtGallery Exports/`，设置中可自定义
- [ ] 导出目录注册为 image_book（books 表加 `is_export_dir` 标记），侧边栏单独分组显示
- [ ] 查看器底栏加"导出"按钮，下拉菜单两种方式：
  - **保存到导出目录**：后端生成图片保存到导出目录，自动刷新 book 记录，app 内可浏览
  - **下载到本地**：浏览器直接下载到系统 Downloads，不进书库
- [ ] "在 Finder 中显示"按钮（仅保存到导出目录时显示）
- [ ] 单页导出 API：
  - `POST /api/books/:id/pages/:filename/export` → 保存到导出目录
  - `GET /api/books/:id/pages/:filename/download` → 浏览器下载
  - image_book：拷贝/透传原图；PDF：mupdf 渲染为 PNG
  - 文件命名：`书名_页码.png`
- [ ] Spread 拼接导出 API：
  - `POST /api/books/:id/spreads/:filename/export` → 保存到导出目录
  - `GET /api/books/:id/spreads/:filename/download` → 浏览器下载
  - 查 page_spreads 拿 next_file / direction / overlap，按 direction 拼合，应用 overlap
  - 文件命名：`书名_左页-右页_spread.png`
- [ ] 导出后自动刷新导出目录的 book 记录（增量添加 page meta）
- [ ] 虚拟书（收藏页）导出：后端根据 source book_type 分别取图再拼

### 5. 收藏页一键导出 ZIP
- [ ] 后端 API：`POST /api/page-favorites/export`
- [ ] 遍历收藏页，收集图片文件路径和 PDF SVG 缓存路径
- [ ] PDF 页面如无 SVG 缓存，即时用 mupdf 渲染
- [ ] ZIP 内结构：`来源书名/页码.ext`，保留来源信息
- [ ] spread 关系记录到 `metadata.json`
- [ ] 异步打包 + 进度推送（SSE 或轮询）
- [ ] 前端：虚拟书详情页或收藏页加"导出"按钮

### 5.5 导出为 CBZ（含 ComicInfo.xml）
- [ ] 任何书籍（image_book）都可导出为 CBZ 格式
- [ ] 自动生成 ComicInfo.xml，填充已有元数据：
  - `reading_direction` → `<Manga>` 字段
  - `page_spreads` → `<Page DoublePage="true">`
  - 书名/分类 → `<Title>` / `<Series>` / `<Genre>`
  - 手动填写的作者/出版社 → `<Writer>` / `<Publisher>`
- [ ] 元数据编辑 UI：信息抽屉加"编辑元数据"（作者/出版社/系列/卷号等）
- [ ] 元数据存储到 `user_data` 表（保持非侵入，不动原文件）
- [ ] 后端 API：`POST /api/books/:id/export-cbz`
- [ ] 完整工作流：原始素材 → 浏览/标记/编辑 → 导出标准化 CBZ → 任何阅读器可用

---

## 优先级 P2 — 体验优化

### 5. CBR 漫画格式支持
- [ ] 引入 `unrar` 解压依赖
- [ ] 扫描识别 `.cbr` 文件，解压后同 CBZ 处理
- [ ] 评估是否值得引入外部依赖（CBR 使用率是否足够高）

### 6. EPUB 格式支持（优先级最低）
- [ ] 解析 EPUB 的 OPF 目录结构
- [ ] 提取图片和页面顺序
- [ ] 考虑是否值得做（EPUB 主要是文字书，和画册场景不匹配）

### 7. RTL 自动检测
- [ ] 扫描时根据文件格式（CBZ/CBR）自动设置 `reading_direction = 'rtl'`
- [ ] 可选：根据文件夹名中的日文特征自动判断

### 8. 统一书籍过滤器
- [ ] 筛选状态定义（format / direction / categoryId / favoriteOnly）
- [ ] 顶部工具栏：格式 QBtnToggle（全部/PDF/图片包/CBZ/CBR）
- [ ] 顶部工具栏：阅读方向 QBtnToggle（全部/LTR/RTL）
- [ ] 前端 computed 过滤书籍列表（当前数据量前端过滤即可）
- [ ] 过滤结果数量显示
- [ ] 将来书多了再推后端过滤（API query 参数）

### 9. 移动端适配（查看器优先级最高）
- [ ] **查看器（最紧急）**：
  - 触屏手势：滑动翻页（Quasar TouchPan 指令）
  - 双指缩放（pinch-to-zoom）
  - 点击区域翻页（左半屏=上一页，右半屏=下一页，RTL 反转）
  - 隐藏桌面端箭头按钮，改为轻触屏幕中央唤出控制栏
  - Spread 双图在窄屏的适配
- [ ] 侧边栏：移动端改为抽屉式（QDrawer behavior: mobile）
- [ ] 瀑布流：移动端强制 2 列，调整间距
- [ ] 详情页：信息抽屉移动端改为全屏覆盖
- [ ] 底栏按钮：加大触摸区域
- [ ] DetailPage 拆分组件：MasonryViewer / ImageViewer / InfoDrawer
- [ ] App 级 `appReady` 状态，统一初始化链路，彻底避免 settings loaded 时序问题
- [ ] 书籍列表展示模式：支持分页/瀑布流切换（已有 grid/list/masonry，缺分页+瀑布流）

### 10. Spread 拼接偏移量
- [ ] `page_spreads` 表新增 `overlap` 字段（INTEGER DEFAULT 0，单位 px）
- [ ] 创建 spread API 新增 `overlap` 参数
- [ ] 查看器底栏拼接操作加偏移量滑块（-50~50px，0=无缝拼接，正=重叠，负=留间隙）
- [ ] 瀑布流 spread 渲染：两张子图重叠 overlap 像素（第二张 margin-left: -overlap）
- [ ] 查看器 spread 渲染：同理
- [ ] 虚拟书 spread 保留 overlap 信息

---

## 技术债务

- [x] `src/frontend/` 构建产物从 git 跟踪中彻底移除（加 `.gitignore`）
- [x] Cargo 编译警告：`strategy.rs` 中 `mtime`/`size` 未使用变量
- [ ] 统一前端状态初始化时机（settings.loaded watch 散落多处）
