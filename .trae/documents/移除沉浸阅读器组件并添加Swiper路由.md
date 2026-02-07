# 移除沉浸阅读器组件并添加Swiper路由

## 任务目标
1. 移除pdf_book_detail和img_book_detail页面的沉浸阅读器组件代码
2. 修改沉浸阅读按钮，使其跳转到新的swiper页面
3. 为swiper组件添加新的RESTful风格路由

## 具体修改计划

### 1. 修改 pdf_book_detail.html
- **移除HTML结构**：删除#scrollReader相关的HTML代码块
- **移除CSS样式**：删除沉浸式滚动模式、浮动工具栏等相关样式
- **移除JavaScript逻辑**：删除openScrollReader、closeScrollReader等相关函数
- **修改按钮行为**：更新btnScrollRead按钮的onclick事件，使其跳转到`/pdf_swiper/{book_id}`
- **修改页面逻辑**：确保book_id变量正确获取，以便在按钮点击时构建正确的URL

### 2. 修改 img_book_detail.html
- **移除HTML结构**：删除#scrollReader相关的HTML代码块
- **移除CSS样式**：删除沉浸式滚动阅读器相关样式
- **移除JavaScript逻辑**：删除startImmersiveReading、closeScrollReader等相关函数
- **修改按钮行为**：更新"开始沉浸阅读"按钮的onclick事件，使其跳转到`/img_swiper/{book_id}`
- **修改页面逻辑**：确保book_id变量正确获取，以便在按钮点击时构建正确的URL

### 3. 修改 img_swiper.html
- **更新路径处理**：修改JavaScript代码，使其从URL路径中提取book_id，而不是从查询参数中获取
- **确保兼容性**：保留对查询参数的支持，以实现向后兼容

### 4. 修改 pdf_swiper.html
- **更新路径处理**：修改JavaScript代码，使其从URL路径中提取book_id，而不是从查询参数中获取
- **确保兼容性**：保留对查询参数的支持，以实现向后兼容

### 5. 修改 app.py
- **添加PDF Swiper路由**：
  - `/pdf_swiper`：返回pdf_swiper.html
  - `/pdf_swiper/{book_id}`：返回pdf_swiper.html并传递book_id参数
- **添加Image Swiper路由**：
  - `/img_swiper`：返回img_swiper.html
  - `/img_swiper/{book_id}`：返回img_swiper.html并传递book_id参数

## 技术要点
- 保持页面其他功能（如缩略图预览）不变
- 确保新的RESTful路由配置正确，支持参数传递
- 确保按钮跳转逻辑正确，构建正确的RESTful风格URL
- 更新swiper页面的路径处理逻辑，支持从URL路径中提取book_id
- 移除冗余代码，保持文件简洁

## 预期结果
- pdf_book_detail和img_book_detail页面不再包含沉浸阅读器组件
- 点击"开始沉浸阅读"按钮时，会跳转到对应的RESTful风格URL的swiper页面
- swiper页面可以通过新的路由访问，并正确从URL路径中提取book_id参数
- 系统保持向后兼容性，支持通过查询参数访问swiper页面