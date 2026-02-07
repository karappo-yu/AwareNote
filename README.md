# Book Scanner App

Book Scanner 是一个功能强大的书籍管理应用，能够自动扫描文件系统中的图片和PDF文件，并将它们组织成书籍。应用提供了直观的网页界面，让用户可以方便地浏览、搜索和管理自己的书籍收藏。

## 功能特点

### 📚 核心功能
- **自动扫描**：自动扫描指定目录中的图片和PDF文件，智能识别书籍结构
- **书籍管理**：支持按分类浏览书籍，查看书籍详情
- **封面生成**：自动为书籍生成封面图片
- **PDF预览**：支持PDF文件的预览功能
- **图片优化**：自动处理超大图片，生成适合浏览的缩略图

### ⭐ 特色功能
- **收藏功能**：支持将书籍添加到收藏，方便快速访问
- **响应式设计**：适配不同屏幕尺寸的设备
- **菜单栏应用**：在macOS上以菜单栏应用形式运行，不占用Dock空间
- **实时扫描**：支持手动触发扫描，实时更新书籍库

### 🛠 技术特性
- **后端**：基于FastAPI构建的高性能API
- **前端**：使用HTML、CSS和JavaScript构建的现代网页界面
- **数据库**：使用SQLite轻量级数据库存储书籍信息
- **线程池**：使用线程池处理耗时任务，提高应用响应速度
- **缓存系统**：实现了高效的缓存系统，提升浏览体验

## 系统要求

- Python 3.10+
- macOS 12.0+
- 足够的磁盘空间用于存储书籍和缓存

## 安装与运行

### 从源代码运行

1. **克隆仓库**
   ```bash
   git clone https://github.com/karappo-yu/book-scanner-app.git
   cd book-scanner-app
   ```

2. **创建虚拟环境**
   ```bash
   python3 -m venv venv
   ```

3. **激活虚拟环境**
   ```bash
   # macOS/Linux
   source venv/bin/activate
   
   # Windows
   venv\Scripts\activate
   ```

4. **安装依赖**
   ```bash
   pip install -r requirements.txt
   ```

5. **配置应用**
   编辑 `config/setting.json` 文件，设置 `root_path` 为你的书籍存储目录

6. **运行应用**
   ```bash
   python entry_point.py
   ```

7. **访问应用**
   应用启动后，在浏览器中访问 `http://localhost:8000`

### 使用打包版本

1. **下载应用**
   从GitHub Releases页面下载最新的打包版本

2. **安装应用**
   - macOS：双击 `.pkg` 文件进行安装
   - 安装完成后，应用会在菜单栏中显示

3. **配置应用**
   首次运行时，需要在设置页面配置扫描目录

4. **开始使用**
   点击菜单栏中的应用图标，选择"打开应用"开始使用

## 配置说明

应用的主要配置项位于 `config/setting.json` 文件中：

- `root_path`：扫描根目录
- `ignored_file_types`：忽略的文件类型
- `cover_width`：封面生成宽度
- `auto_scan_on_startup`：启动时是否自动扫描
- `use_thread_pool`：是否使用线程池
- `thread_pool_max_workers`：线程池最大工作线程数
- `thread_pool_idle_timeout`：线程池空闲超时时间

## 项目结构

```
book-scanner-app/
├── app.py              # 主应用入口
├── entry_point.py      # 应用入口点（用于打包）
├── config/             # 配置文件
├── database/           # 数据库相关代码
├── models/             # 数据模型
├── routers/            # API路由
├── scanner/            # 文件扫描器
├── static/             # 前端静态文件
├── utils/              # 工具函数
├── icon/               # 应用图标
└── requirements.txt    # 依赖文件
```

## 开发指南

### 环境设置

1. 按照"从源代码运行"部分的步骤设置开发环境

2. 安装开发依赖
   ```bash
   pip install -r requirements.txt
   ```

### 代码风格

- 遵循 PEP 8 代码风格规范
- 使用类型提示增强代码可读性
- 编写清晰的文档字符串

### 贡献指南

1. Fork 本仓库
2. 创建功能分支
3. 提交更改
4. 推送到分支
5. 开启 Pull Request

## 许可证

本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。

## 联系方式

- GitHub: [karappo-yu](https://github.com/karappo-yu)

## 更新日志

### v1.2.0
- 新增收藏功能
- 改进扫描算法
- 优化前端界面
- 修复已知问题

### v1.1.0
- 新增PDF预览功能
- 改进封面生成算法
- 优化缓存系统

### v1.0.0
- 初始版本
- 基本的书籍扫描和管理功能
- 响应式网页界面
