from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from fastapi.responses import FileResponse
import os

# 导入扫描器和数据库
from scanner.scanner import Scanner
from database.db import db

# 导入配置
from config.config import auto_scan_on_startup

# 创建 Fast API 应用
app = FastAPI(
    title="Book Scanner API",
    description="书籍和分类的管理 API",
    version="1.0.0"
)

# 挂载静态文件目录
static_dir = os.path.join(os.path.dirname(__file__), "static")
if os.path.exists(static_dir):
    app.mount("/static", StaticFiles(directory=static_dir), name="static")

# 配置 CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # 在生产环境中应该设置具体的域名
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

def sync_data():
    """执行扫描并同步数据
    
    Returns:
        dict: 扫描和同步结果
    """
    try:
        # 创建扫描器实例并执行扫描
        scanner = Scanner()
        root_category = scanner.scan()
        
        # 从根分类中提取所有分类和书籍
        def extract_categories_and_books(category):
            all_categories = []
            all_books = []
            
            def traverse(current):
                all_categories.append(current)
                all_books.extend(current.books)
                for sub in current.sub_categories:
                    traverse(sub)
            
            traverse(category)
            return all_categories, all_books
        
        scanned_categories, scanned_books = extract_categories_and_books(root_category)
        
        # 获取数据库中现有的书籍和分类
        db_books = db.get_all_books()
        db_categories = db.get_all_categories()
        
        # 构建 ID 集合用于快速查找
        scanned_book_ids = {book.id for book in scanned_books}
        scanned_category_ids = {category.id for category in scanned_categories}
        db_book_ids = {book.id for book in db_books}
        db_category_ids = {category.id for category in db_categories}
        
        # 同步书籍
        added_books = 0
        updated_books = 0
        deleted_books = 0
        
        # 添加新书籍或更新现有书籍
        for book in scanned_books:
            if book.id not in db_book_ids:
                try:
                    db.add_book(book)
                    added_books += 1
                    print(f"添加书籍: {book.title}")
                    
                    # 生成书籍封面
                    try:
                        from utils.image_utils import generate_cover
                        generate_cover(book)
                        print(f"生成封面: {book.title}")
                    except Exception as e:
                        print(f"生成封面失败 {book.title}: {e}")
                        # 封面生成失败不影响书籍添加
                except Exception as e:
                    print(f"添加书籍失败 {book.title}: {e}")
            else:
                try:
                    updated = db.update_book(book)
                    updated_books += updated
                    if updated:
                        print(f"更新书籍: {book.title}")
                except Exception as e:
                    print(f"更新书籍失败 {book.title}: {e}")
        
        # 删除不存在的书籍
        for book in db_books:
            if book.id not in scanned_book_ids:
                try:
                    db.delete_book(book.id)
                    deleted_books += 1
                    print(f"删除书籍: {book.title}")
                except Exception as e:
                    print(f"删除书籍失败 {book.title}: {e}")
        
        # 同步分类
        added_categories = 0
        deleted_categories = 0
        
        # 添加新分类或更新现有分类
        for category in scanned_categories:
            if category.id not in db_category_ids:
                try:
                    db.add_category(category)
                    added_categories += 1
                    print(f"添加分类: {category.name}")
                except Exception as e:
                    print(f"添加分类失败 {category.name}: {e}")
            else:
                try:
                    db.update_category(category)
                    print(f"更新分类: {category.name}")
                except Exception as e:
                    print(f"更新分类失败 {category.name}: {e}")
        
        # 删除不存在的分类
        for category in db_categories:
            if category.id not in scanned_category_ids:
                try:
                    db.delete_category(category.id)
                    deleted_categories += 1
                    print(f"删除分类: {category.name}")
                except Exception as e:
                    print(f"删除分类失败 {category.name}: {e}")
        
        # 创建分类与分类、分类与书籍的关系
        def create_relations(category):
            """递归创建分类与子分类、分类与书籍的关系"""
            # 创建分类与子分类的关系
            for sub_category in category.sub_categories:
                db.add_entity_relation(category.id, sub_category.id, 'category_category')
                # 递归处理子分类
                create_relations(sub_category)
            
            # 创建分类与书籍的关系（只关联直接子书籍）
            for book in category.books:
                db.add_entity_relation(category.id, book.id, 'category_book')
        
        # 为根分类创建关系
        create_relations(root_category)
        
        # 清空并重建缓存（从数据库构建，以包含正确的 created_at 字段）
        db.clear_cache()
        db.build_cache()
        
        # 构建结果
        result = {
            "scanned": {
                "categories": len(scanned_categories),
                "books": len(scanned_books)
            },
            "synced": {
                "added_categories": added_categories,
                "deleted_categories": deleted_categories,
                "added_books": added_books,
                "updated_books": updated_books,
                "deleted_books": deleted_books
            },
            "database": {
                "categories": len(db.get_all_categories()),
                "books": len(db.get_all_books())
            }
        }
        
        return result
        
    except Exception as e:
        print(f"扫描和同步过程中出错: {e}")
        return {"error": str(e)}

# 启动时执行扫描并同步数据
@app.on_event("startup")
async def startup_event():
    """应用启动时执行扫描并同步数据"""
    if auto_scan_on_startup:
        print("应用启动中，开始扫描文件系统...")
        
        result = sync_data()
        
        if "error" not in result:
            print("数据同步完成！")
            print(f"扫描结果: {result['scanned']['categories']} 个分类, {result['scanned']['books']} 本书籍")
            print(f"数据库状态: {result['database']['categories']} 个分类, {result['database']['books']} 本书籍")
        else:
            print(f"扫描和同步过程中出错: {result['error']}")
    else:
        print("应用启动中，跳过自动扫描（配置为不自动扫描）...")
        # 构建缓存（从数据库构建，以确保缓存初始化）
        db.clear_cache()
        db.build_cache()

# 关闭时清理资源
@app.on_event("shutdown")
async def shutdown_event():
    """应用关闭时清理资源"""
    print("应用关闭中，清理资源...")
    
    # 关闭全局进程池
    try:
        from utils.pool_manager import shutdown_thread_pool
        shutdown_thread_pool()
        print("关闭全局进程池成功")
    except Exception as e:
        print(f"关闭全局进程池失败: {e}")
    
    print("资源清理完成")

# 注册路由
from routers import book, category, custom_category, settings

app.include_router(book.router, prefix="/api/books", tags=["books"])
app.include_router(category.router, prefix="/api/categories", tags=["categories"])
app.include_router(custom_category.router, prefix="/api/custom-categories", tags=["custom-categories"])
app.include_router(settings.router, prefix="/api", tags=["settings"])

# 根路径 - 返回前端首页
@app.get("/")
async def root():
    index_path = os.path.join(static_dir, "index.html")
    if os.path.exists(index_path):
        return FileResponse(index_path)
    return {"message": "Welcome to Book Scanner API"}
# 前端页面路由
@app.get("/config")
async def get_config_page():
    config_path = os.path.join(static_dir, "config.html")
    if os.path.exists(config_path):
        return FileResponse(config_path)
    return {"error": "Config page not found"}

@app.get("/img_book_detail")
async def get_img_book_detail_page():
    detail_path = os.path.join(static_dir, "img_book_detail.html")
    if os.path.exists(detail_path):
        return FileResponse(detail_path)
    return {"error": "Image book detail page not found"}

@app.get("/pdf_book_detail")
async def get_pdf_book_detail_page():
    detail_path = os.path.join(static_dir, "pdf_book_detail.html")
    if os.path.exists(detail_path):
        return FileResponse(detail_path)
    return {"error": "PDF book detail page not found"}

@app.get("/scorll")
async def get_scorll_page():
    scorll_path = os.path.join(static_dir, "scorll.html")
    if os.path.exists(scorll_path):
        return FileResponse(scorll_path)
    return {"error": "Scorll page not found"}

# 图片书籍详情页面路由
@app.get("/img_book_detail/{book_id}")
async def get_img_book_detail_page(book_id: str):
    detail_path = os.path.join(static_dir, "img_book_detail.html")
    if os.path.exists(detail_path):
        return FileResponse(detail_path)
    return {"error": "Image book detail page not found"}

# PDF书籍详情页面路由
@app.get("/pdf_book_detail/{book_id}")
async def get_pdf_book_detail_page(book_id: str):
    detail_path = os.path.join(static_dir, "pdf_book_detail.html")
    if os.path.exists(detail_path):
        return FileResponse(detail_path)
    return {"error": "PDF book detail page not found"}

# PDF Swiper 路由
@app.get("/pdf_swiper")
async def get_pdf_swiper_page():
    swiper_path = os.path.join(static_dir, "pdf_swiper.html")
    if os.path.exists(swiper_path):
        return FileResponse(swiper_path)
    return {"error": "PDF swiper page not found"}

@app.get("/pdf_swiper/{book_id}")
async def get_pdf_swiper_page(book_id: str):
    swiper_path = os.path.join(static_dir, "pdf_swiper.html")
    if os.path.exists(swiper_path):
        return FileResponse(swiper_path)
    return {"error": "PDF swiper page not found"}

# Image Swiper 路由
@app.get("/img_swiper")
async def get_img_swiper_page():
    swiper_path = os.path.join(static_dir, "img_swiper.html")
    if os.path.exists(swiper_path):
        return FileResponse(swiper_path)
    return {"error": "Image swiper page not found"}

@app.get("/img_swiper/{book_id}")
async def get_img_swiper_page(book_id: str):
    swiper_path = os.path.join(static_dir, "img_swiper.html")
    if os.path.exists(swiper_path):
        return FileResponse(swiper_path)
    return {"error": "Image swiper page not found"}

from fastapi.responses import StreamingResponse
import asyncio

# 手动触发扫描（SSE 版本）
@app.get("/api/scan")
async def trigger_scan():
    """手动触发扫描并同步数据（SSE 版本）"""
    async def event_generator():
        try:
            # 发送开始扫描的消息
            yield "data: {\"type\": \"INFO\", \"message\": \"Starting file system scan...\"}\n\n"
            
            # 导入扫描器
            from scanner.scanner import Scanner
            scanner = Scanner()
            
            # 执行扫描
            yield "data: {\"type\": \"INFO\", \"message\": \"Scanning directories...\"}\n\n"
            root_category = scanner.scan()
            
            # 从根分类中提取所有分类和书籍
            def extract_categories_and_books(category):
                all_categories = []
                all_books = []
                
                def traverse(current):
                    all_categories.append(current)
                    all_books.extend(current.books)
                    for sub in current.sub_categories:
                        traverse(sub)
                
                traverse(category)
                return all_categories, all_books
            
            # scanned_categories 已经是所有的分类（包括子分类）的扁平化列表
            scanned_categories, scanned_books = extract_categories_and_books(root_category)
            
            # 获取数据库中现有的书籍和分类
            db_books = db.get_all_books()
            db_categories = db.get_all_categories()
            
            # 构建 ID 集合用于快速查找
            scanned_book_ids = {book.id for book in scanned_books}
            
            # 这里的 scanned_categories 已经包含了所有分类，不需要再递归获取
            # 如果再递归获取，会导致子分类在列表中出现多次（父分类带子分类，子分类自己又出现一次）
            # 这就是导致第一次成功，第二次报 IntegrityError 的原因
            
            scanned_category_ids = {category.id for category in scanned_categories}
            
            # 构建数据库书籍和分类的ID集合
            db_book_ids = {book.id for book in db_books}
            
            # 递归获取数据库中所有分类（包括子分类）的ID
            def get_all_category_ids_recursive(categories):
                category_ids = []
                for category in categories:
                    category_ids.append(category.id)
                    if category.sub_categories:
                        category_ids.extend(get_all_category_ids_recursive(category.sub_categories))
                return category_ids
            
            # 获取数据库中所有分类的ID
            db_category_ids = set(get_all_category_ids_recursive(db_categories))
            
            added_books = 0
            updated_books = 0
            deleted_books = 0
            added_categories = 0
            deleted_categories = 0
            
            # 添加新书籍或更新现有书籍
            for book in scanned_books:
                if book.id not in db_book_ids:
                    try:
                        db.add_book(book)
                        added_books += 1
                        yield f"data: {{\"type\": \"SUCCESS\", \"message\": \"Added book: {book.title}\"}}\n\n"
                        
                        # 生成书籍封面
                        try:
                            from utils.image_utils import generate_cover
                            generate_cover(book)
                            yield f"data: {{\"type\": \"INFO\", \"message\": \"Generated cover for: {book.title}\"}}\n\n"
                        except Exception as e:
                            yield f"data: {{\"type\": \"WARN\", \"message\": \"Failed to generate cover for {book.title}: {str(e)}\"}}\n\n"
                    except Exception as e:
                        yield f"data: {{\"type\": \"WARN\", \"message\": \"Failed to add book {book.title}: {str(e)}\"}}\n\n"
                else:
                    try:
                        updated = db.update_book(book)
                        updated_books += updated
                        if updated:
                            yield f"data: {{\"type\": \"INFO\", \"message\": \"Updated book: {book.title}\"}}\n\n"
                    except Exception as e:
                        yield f"data: {{\"type\": \"WARN\", \"message\": \"Failed to update book {book.title}: {str(e)}\"}}\n\n"
            
            # 删除不存在的书籍
            for book in db_books:
                if book.id not in scanned_book_ids:
                    try:
                        db.delete_book(book.id)
                        deleted_books += 1
                        yield f"data: {{\"type\": \"INFO\", \"message\": \"Deleted book: {book.title}\"}}\n\n"
                        
                        # 删除书籍相关缓存
                        try:
                            from utils.cache_utils import clear_book_cache
                            clear_book_cache(book.id)
                            yield f"data: {{\"type\": \"INFO\", \"message\": \"Cleared cache for book: {book.title}\"}}\n\n"
                        except Exception as cache_error:
                            yield f"data: {{\"type\": \"WARN\", \"message\": \"Failed to clear cache for book {book.title}: {str(cache_error)}\"}}\n\n"
                    except Exception as e:
                        yield f"data: {{\"type\": \"WARN\", \"message\": \"Failed to delete book {book.title}: {str(e)}\"}}\n\n"
            
            # 添加新分类或更新现有分类
            # 修正：直接使用 scanned_categories 迭代，而不是 all_scanned_categories
            for category in scanned_categories:
                if category.id not in db_category_ids:
                    try:
                        db.add_category(category)
                        added_categories += 1
                        yield f"data: {{\"type\": \"SUCCESS\", \"message\": \"Added category: {category.name}\"}}\n\n"
                    except Exception as e:
                        yield f"data: {{\"type\": \"WARN\", \"message\": \"Failed to add category {category.name}: {str(e)}\"}}\n\n"
                else:
                    try:
                        # 检查分类内容是否真的有变化
                        existing_category = db.get_category_by_id(category.id)
                        if existing_category:
                            # 比较关键属性是否有变化
                            if (existing_category.name != category.name or 
                                existing_category.path != category.path):
                                db.update_category(category)
                                yield f"data: {{\"type\": \"INFO\", \"message\": \"Updated category: {category.name}\"}}\n\n"
                        else:
                            # 如果分类不存在（可能是缓存问题），则添加
                            db.add_category(category)
                            added_categories += 1
                            yield f"data: {{\"type\": \"SUCCESS\", \"message\": \"Added category: {category.name}\"}}\n\n"
                    except Exception as e:
                        yield f"data: {{\"type\": \"WARN\", \"message\": \"Failed to update category {category.name}: {str(e)}\"}}\n\n"
            
            # 删除不存在的分类
            for category in db_categories:
                if category.id not in scanned_category_ids:
                    try:
                        db.delete_category(category.id)
                        deleted_categories += 1
                        yield f"data: {{\"type\": \"INFO\", \"message\": \"Deleted category: {category.name}\"}}\n\n"
                    except Exception as e:
                        yield f"data: {{\"type\": \"WARN\", \"message\": \"Failed to delete category {category.name}: {str(e)}\"}}\n\n"
            
            # 创建分类与分类、分类与书籍的关系
            def create_relations(category):
                """递归创建分类与子分类、分类与书籍的关系"""
                # 创建分类与子分类的关系
                for sub_category in category.sub_categories:
                    db.add_entity_relation(category.id, sub_category.id, 'category_category')
                    # 递归处理子分类
                    create_relations(sub_category)
                
                # 创建分类与书籍的关系（只关联直接子书籍）
                for book in category.books:
                    db.add_entity_relation(category.id, book.id, 'category_book')
            
            # 为根分类创建关系
            create_relations(root_category)
            
            # 清空并重建缓存（从数据库构建，以包含正确的 created_at 字段）
            db.clear_cache()
            db.build_cache()
            
            # 构建结果
            result = {
                "scanned": {
                    "categories": len(scanned_categories),
                    "books": len(scanned_books)
                },
                "synced": {
                    "added_categories": added_categories,
                    "deleted_categories": deleted_categories,
                    "added_books": added_books,
                    "updated_books": updated_books,
                    "deleted_books": deleted_books
                },
                "database": {
                    "categories": len(db.get_all_categories()),
                    "books": len(db.get_all_books())
                }
            }
            
            # 构建变更统计
            changes = []
            if added_books > 0:
                changes.append(f"{added_books} new books")
            if updated_books > 0:
                changes.append(f"{updated_books} updated books")
            if deleted_books > 0:
                changes.append(f"{deleted_books} deleted books")
            if added_categories > 0:
                changes.append(f"{added_categories} new categories")
            if deleted_categories > 0:
                changes.append(f"{deleted_categories} deleted categories")
            
            # 构建完成消息
            if changes:
                message = f"Scan finished. Detected changes: {', '.join(changes)}"
            else:
                message = "Scan finished. No changes detected."
            
            # 构建扫描总结信息
            summary_message = f"Scanned {len(scanned_books)} books. Added: {added_books}, Updated: {updated_books}, Deleted: {deleted_books}"
            yield f"data: {{\"type\": \"INFO\", \"message\": \"{summary_message}\"}}\n\n"
            
            # 发送完成消息
            yield f"data: {{\"type\": \"COMPLETE\", \"message\": \"{message}\", \"status\": \"ok\"}}\n\n"
            
        except Exception as e:
            # 发送错误消息
            error_msg = f"Scan failed: {str(e)}"
            yield f"data: {{\"type\": \"COMPLETE\", \"message\": \"{error_msg}\", \"status\": \"error\"}}\n\n"
    
    # 返回 StreamingResponse
    return StreamingResponse(
        event_generator(),
        media_type="text/event-stream"
    )