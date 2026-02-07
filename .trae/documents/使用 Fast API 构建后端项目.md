# 使用 Fast API 构建后端项目

## 项目结构

我将在现有的目录结构基础上添加 Fast API 相关的文件和目录：

```
/Users/mac/Downloads/scan demo/
  ├── config/
  │   └── setting.json
  ├── models/
  │   ├── book.py
  │   └── category.py
  ├── scanner/
  │   └── scanner.py
  ├── main.py
  ├── app.py          # Fast API 应用入口点
  ├── requirements.txt # 依赖项文件
  ├── routers/
  │   ├── book.py     # 书籍相关 API 路由
  │   └── category.py # 分类相关 API 路由
  └── database/
      ├── __init__.py
      └── db.py       # 数据库连接和操作
```

## 实施步骤

1. **创建 requirements.txt 文件**

   * 添加 Fast API 相关的依赖项，包括 fastapi、uvicorn、pydantic 等

2. **创建 app.py 文件**

   * 初始化 Fast API 应用

   * 配置 CORS

   * 注册路由

3. **创建 routers 目录和路由文件**

   * 创建 book.py，实现书籍的增删查改 API

   * 创建 category.py，实现分类的增删查改 API

4. **创建 database 目录和数据库文件**

   * 创建 db.py，实现数据库连接和操作

   * 使用内存数据库作为临时存储（可后续扩展为持久化数据库）

5. **修改模型文件**

   * 确保 Book 和 Category 模型与 Fast API 兼容

   * 添加 Pydantic 模型用于请求和响应验证

6. **实现 API 端点**

   * 书籍 API：GET /books, GET /books/{id}, POST /books, PUT /books/{id}, DELETE /books/{id}

   * 分类 API：GET /categories, GET /categories/{id}, POST /categories, PUT /categories/{id}, DELETE /categories/{id}

7. **测试 API**

   * 使用 Fast API 的自动文档功能测试 API 是否正常工作

## 技术选型

* **Web 框架**：Fast API

* **ASGI 服务器**：Uvicorn

* **数据验证**：Pydantic

* **临时存储**：内存数据库（可后续扩展为 SQLite、PostgreSQL 等）

* **CORS**：Fast API 内置的 CORS 中间件

## 预期结果

* 成功启动 Fast API 服务器

* 实现对 Book 和 Category 模型的增删查改 API

* 通过自动生成的 API 文档测试所有端点

* 扫描功能与 API 集成，可通过 API 触发扫描并获取结果

