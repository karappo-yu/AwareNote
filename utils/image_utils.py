import os
import sys
import json
import asyncio
from PIL import Image
import fitz  # PyMuPDF for PDF processing

# 增加 PIL 图像大小限制，防止处理大 PDF 时出现 "decompression bomb" 错误
Image.MAX_IMAGE_PIXELS = 1000000000  # 设置为 10 亿像素，足够处理大多数 PDF

# 添加项目根目录到导入路径
sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))

from database.db import get_db

# 获取配置信息
from config.config import cover_width, get

# 导入全局进程池管理模块
from utils.pool_manager import get_thread_pool, shutdown_thread_pool, get_processing_tasks, get_processing_lock

# 正在处理的任务集合，用于避免重复任务
_processing_tasks = get_processing_tasks()
_processing_lock = get_processing_lock()

# 解耦的处理函数 - 不依赖任何 FastAPI 对象，纯同步逻辑
def process_image_to_cover(image_path: str, cover_path: str, target_width: int) -> str:
    """
    处理图片并保存为封面（纯同步函数，用于进程池）
    
    参数:
        image_path: 原始图片路径
        cover_path: 封面保存路径
        target_width: 目标宽度
    返回:
        cover_path: 封面路径
    """
    with Image.open(image_path) as img:
        # 1. 趁数据还没读进内存，赶紧 draft
        img.draft('RGB', (target_width, target_width * 2))
        
        # 2. 处理色彩模式与透明度
        if img.mode in ('RGBA', 'LA') or (img.mode == 'P' and 'transparency' in img.info):
            # 统一转为有透明度的 RGBA 方便处理
            img = img.convert('RGBA')
            # 创建白色背景底图
            background = Image.new('RGB', img.size, (255, 255, 255))
            # 精致写法：直接用 img 充当自己的 mask，避免 split() 带来的计算开销
            background.paste(img, (0, 0), img)
            img = background
        elif img.mode != 'RGB':
            img = img.convert('RGB')
        
        # 3. 重新获取 draft 后的 size 进行比例计算
        w, h = img.size
        target_height = int((target_width / w) * h)
        
        # 4. 原地缩放
        img.thumbnail((target_width, target_height), Image.Resampling.LANCZOS)
        
        # 5. 保存，使用 optimize=True 优化文件大小
        img.save(cover_path, "JPEG", quality=85, optimize=True)
    
    return cover_path

def process_pdf_to_cover(pdf_path: str, cover_path: str, target_width: int) -> str:
    """
    处理 PDF 并保存为封面（纯同步函数，用于进程池）
    
    优化点：
    - 使用 page.get_pixmap(matrix=fitz.Matrix(zoom, zoom)) 直接渲染出合适大小的像素
    - 使用 Image.frombytes 在内存中直接把像素转成 Pillow 对象
    - 彻底省掉 PDF 处理时的磁盘 IO 损耗
    
    参数:
        pdf_path: PDF 文件路径
        cover_path: 封面保存路径
        target_width: 目标宽度
    返回:
        cover_path: 封面路径
    """
    with fitz.open(pdf_path) as pdf:
        if pdf.page_count > 0:
            page = pdf[0]  # 获取第一页
            
            # 计算合适的缩放比例，直接渲染出目标大小的像素
            # 首先获取页面原始尺寸（以点为单位的矩形）
            rect = page.rect
            page_width = rect.width
            
            # 计算缩放比例：目标宽度 / 页面原始宽度
            # 72 DPI 是 PDF 的默认分辨率，1 点 = 1/72 英寸
            zoom = target_width / page_width * 2  # 乘以2以获得更好的质量，然后缩放
            
            # 使用 matrix 直接渲染出合适大小的像素，避免后续缩放
            mat = fitz.Matrix(zoom, zoom)
            pix = page.get_pixmap(matrix=mat)
            
            # 使用 Image.frombytes 在内存中直接把像素转成 Pillow 对象
            # pix.samples 是原始字节数据，pix.width 和 pix.height 是尺寸
            img = Image.frombytes("RGB", [pix.width, pix.height], pix.samples)
            
            # 如果渲染出来的图片比目标大，进行缩放
            if img.width > target_width:
                # 计算新高度，保持宽高比
                new_height = int((target_width / img.width) * img.height)
                # 原地缩放
                img.thumbnail((target_width, new_height), Image.Resampling.LANCZOS)
            
            # 保存为封面
            img.save(cover_path, "JPEG", quality=85, optimize=True)
    
    return cover_path

def process_image_to_thumbnail(image_path: str, thumbnail_path: str, target_width: int) -> str:
    """
    处理图片并保存为缩略图（纯同步函数，用于进程池）
    
    参数:
        image_path: 原始图片路径
        thumbnail_path: 缩略图保存路径
        target_width: 目标宽度
    返回:
        thumbnail_path: 缩略图路径
    """
    # 检查文件扩展名
    ext = os.path.splitext(image_path)[1].lower()
    
    if ext == '.svg':
        # 处理 SVG 文件
        try:
            # 使用 cairosvg 或其他库转换 SVG 为图片
            # 这里我们使用一个简单的方法，将 SVG 渲染为 PNG 然后处理
            import cairosvg
            import io
            
            # 读取 SVG 文件内容
            with open(image_path, 'r', encoding='utf-8') as f:
                svg_content = f.read()
            
            # 转换 SVG 为 PNG
            png_data = cairosvg.svg2png(bytestring=svg_content.encode('utf-8'))
            
            # 使用 PIL 打开 PNG 数据
            with Image.open(io.BytesIO(png_data)) as img:
                # 1. 趁数据还没读进内存，赶紧 draft
                img.draft('RGB', (target_width, target_width * 2))
                
                # 2. 处理色彩模式与透明度
                if img.mode in ('RGBA', 'LA') or (img.mode == 'P' and 'transparency' in img.info):
                    # 统一转为有透明度的 RGBA 方便处理
                    img = img.convert('RGBA')
                    # 创建白色背景底图
                    background = Image.new('RGB', img.size, (255, 255, 255))
                    # 精致写法：直接用 img 充当自己的 mask，避免 split() 带来的计算开销
                    background.paste(img, (0, 0), img)
                    img = background
                elif img.mode != 'RGB':
                    img = img.convert('RGB')
                
                # 3. 重新获取 draft 后的 size 进行比例计算
                w, h = img.size
                target_height = int((target_width / w) * h)
                
                # 4. 原地缩放
                img.thumbnail((target_width, target_height), Image.Resampling.LANCZOS)
                
                # 5. 保存，使用 optimize=True 优化文件大小
                img.save(thumbnail_path, "JPEG", quality=80, optimize=True)
        except ImportError:
            # 如果没有安装 cairosvg，使用一个简单的占位符
            print("cairosvg not installed, using placeholder for SVG")
            # 创建一个白色背景的占位符图片
            img = Image.new('RGB', (target_width, int(target_width * 1.4)), (255, 255, 255))
            img.save(thumbnail_path, "JPEG", quality=80, optimize=True)
        except Exception as e:
            print(f"Error processing SVG: {e}")
            # 创建一个白色背景的占位符图片
            img = Image.new('RGB', (target_width, int(target_width * 1.4)), (255, 255, 255))
            img.save(thumbnail_path, "JPEG", quality=80, optimize=True)
    else:
        # 处理常规图片文件
        with Image.open(image_path) as img:
            # 1. 趁数据还没读进内存，赶紧 draft
            img.draft('RGB', (target_width, target_width * 2))
            
            # 2. 处理色彩模式与透明度
            if img.mode in ('RGBA', 'LA') or (img.mode == 'P' and 'transparency' in img.info):
                # 统一转为有透明度的 RGBA 方便处理
                img = img.convert('RGBA')
                # 创建白色背景底图
                background = Image.new('RGB', img.size, (255, 255, 255))
                # 精致写法：直接用 img 充当自己的 mask，避免 split() 带来的计算开销
                background.paste(img, (0, 0), img)
                img = background
            elif img.mode != 'RGB':
                img = img.convert('RGB')
            
            # 3. 重新获取 draft 后的 size 进行比例计算
            w, h = img.size
            target_height = int((target_width / w) * h)
            
            # 4. 原地缩放
            img.thumbnail((target_width, target_height), Image.Resampling.LANCZOS)
            
            # 5. 保存，使用 optimize=True 优化文件大小
            img.save(thumbnail_path, "JPEG", quality=80, optimize=True)
    
    return thumbnail_path

# 生成书籍封面（异步版本）
async def generate_cover_async(book) -> str:
    """
    异步生成书籍封面
    
    参数:
        book: 书籍对象
    返回:
        cover_path: 封面图片路径
    """
    if not book:
        raise ValueError("Book object is required")
    
    # 确保cache/covers目录存在
    cache_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "cache", "covers")
    os.makedirs(cache_dir, exist_ok=True)
    
    # 封面图片路径
    cover_filename = f"{book.id}.jpg"
    cover_path = os.path.join(cache_dir, cover_filename)
    
    # 如果封面已存在，直接返回
    if os.path.exists(cover_path):
        return cover_path
    
    # 创建任务标识
    task_id = f"cover:{book.id}"
    
    # 检查是否已有相同任务在处理中
    with _processing_lock:
        if task_id in _processing_tasks:
            # 等待现有任务完成
            while task_id in _processing_tasks:
                await asyncio.sleep(0.1)
            # 再次检查封面是否已生成
            if os.path.exists(cover_path):
                return cover_path
        _processing_tasks.add(task_id)
    
    try:
        from utils.pool_manager import submit_task_to_pool
        
        if book.type == "pdf_book":
            # 使用新的submit_task_to_pool函数提交任务
            cover_path = await submit_task_to_pool(
                process_pdf_to_cover,
                book.path,
                cover_path,
                cover_width
            )
        
        elif book.type == "image_book":
            # 处理图片包书籍
            if hasattr(book, "pages") and book.pages:
                first_page_path = book.pages[0]
                if os.path.exists(first_page_path):
                    # 使用新的submit_task_to_pool函数提交任务
                    cover_path = await submit_task_to_pool(
                        process_image_to_cover,
                        first_page_path,
                        cover_path,
                        cover_width
                    )
                else:
                    raise ValueError(f"Image path does not exist: {first_page_path}")
            else:
                raise ValueError("Image book has no pages")
        
        else:
            raise ValueError(f"Unsupported book type: {book.type}")
        
        # 检查封面是否生成成功
        if not os.path.exists(cover_path):
            raise ValueError("Failed to generate cover")
        
        return cover_path
    
    except Exception as e:
        print(f"Error generating cover for book {book.id}: {e}")
        raise
    
    finally:
        # 从处理集合中移除
        with _processing_lock:
            _processing_tasks.discard(task_id)

# 生成书籍封面（同步版本，保持向后兼容）
def generate_cover_sync(book) -> str:
    """
    生成书籍封面（同步版本，保持向后兼容）
    
    参数:
        book: 书籍对象
    返回:
        cover_path: 封面图片路径
    """
    if not book:
        raise ValueError("Book object is required")
    
    # 确保cache/covers目录存在
    cache_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "cache", "covers")
    os.makedirs(cache_dir, exist_ok=True)
    
    # 封面图片路径
    cover_filename = f"{book.id}.jpg"
    cover_path = os.path.join(cache_dir, cover_filename)
    
    # 如果封面已存在，直接返回
    if os.path.exists(cover_path):
        return cover_path
    
    try:
        if book.type == "pdf_book":
            # 处理PDF书籍
            return process_pdf_to_cover(book.path, cover_path, cover_width)
        
        elif book.type == "image_book":
            # 处理图片包书籍
            if hasattr(book, "pages") and book.pages:
                first_page_path = book.pages[0]
                if os.path.exists(first_page_path):
                    return process_image_to_cover(first_page_path, cover_path, cover_width)
                else:
                    raise ValueError(f"Image path does not exist: {first_page_path}")
            else:
                raise ValueError("Image book has no pages")
        
        else:
            raise ValueError(f"Unsupported book type: {book.type}")
    
    except Exception as e:
        print(f"Error generating cover for book {book.id}: {e}")
        raise

# 生成书籍封面（通用版本，根据配置自动选择同步或异步）
async def generate_cover(book) -> str:
    """
    生成书籍封面（通用版本，根据配置自动选择同步或异步）
    
    参数:
        book: 书籍对象
    返回:
        cover_path: 封面图片路径
    """
    use_process_pool = get("use_process_pool", False)
    
    if use_process_pool:
        return await generate_cover_async(book)
    else:
        return generate_cover_sync(book)

# 生成书籍页码缩略图（异步版本）
async def generate_thumbnail_async(book, page_number: int, thumbnail_width: int) -> str:
    """
    异步生成书籍指定页码的缩略图
    
    参数:
        book: 书籍对象
        page_number: 页码
        thumbnail_width: 缩略图宽度
    返回:
        thumbnail_path: 缩略图路径
    """
    # 确保cache/thumbnails目录存在
    cache_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "cache", "thumbnails", book.id)
    os.makedirs(cache_dir, exist_ok=True)
    
    # 缩略图路径
    thumbnail_filename = f"{page_number}_{thumbnail_width}.jpg"
    thumbnail_path = os.path.join(cache_dir, thumbnail_filename)
    
    # 如果缩略图已存在，直接返回
    if os.path.exists(thumbnail_path):
        return thumbnail_path
    
    # 创建任务标识
    task_id = f"thumbnail:{book.id}:{page_number}:{thumbnail_width}"
    
    # 检查是否已有相同任务在处理中
    with _processing_lock:
        if task_id in _processing_tasks:
            # 等待现有任务完成
            while task_id in _processing_tasks:
                await asyncio.sleep(0.1)
            # 再次检查缩略图是否已生成
            if os.path.exists(thumbnail_path):
                return thumbnail_path
        _processing_tasks.add(task_id)
    
    try:
        from utils.pool_manager import submit_task_to_pool
        
        if book.type == "pdf_book":
            # 处理PDF书籍
            from utils.pdf_utils import get_pdf_page_svg_async
            
            # 获取PDF页面SVG（会自动处理缓存）
            pdf_page_svg = await get_pdf_page_svg_async(book.path, book.id, page_number)
            
            # 使用新的submit_task_to_pool函数提交任务
            thumbnail_path = await submit_task_to_pool(
                process_image_to_thumbnail,
                pdf_page_svg,
                thumbnail_path,
                thumbnail_width
            )
        
        elif book.type == "image_book":
            # 处理图片包书籍
            if hasattr(book, "pages") and book.pages:
                if 0 < page_number <= len(book.pages):
                    page_path = book.pages[page_number - 1]
                    if os.path.exists(page_path):
                        # 使用新的submit_task_to_pool函数提交任务
                        thumbnail_path = await submit_task_to_pool(
                            process_image_to_thumbnail,
                            page_path,
                            thumbnail_path,
                            thumbnail_width
                        )
                    else:
                        raise ValueError(f"Image path does not exist: {page_path}")
                else:
                    raise ValueError(f"Page number {page_number} out of range")
            else:
                raise ValueError("Image book has no pages")
        
        else:
            raise ValueError(f"Unsupported book type: {book.type}")
        
        # 检查缩略图是否生成成功
        if not os.path.exists(thumbnail_path):
            raise ValueError("Failed to generate thumbnail")
        
        return thumbnail_path
    
    except Exception as e:
        print(f"Error generating thumbnail for book {book.id}, page {page_number}: {e}")
        raise
    
    finally:
        # 从处理集合中移除
        with _processing_lock:
            _processing_tasks.discard(task_id)

# 生成书籍页码缩略图（同步版本，保持向后兼容）
def generate_thumbnail_sync(book, page_number: int, thumbnail_width: int) -> str:
    """
    生成书籍指定页码的缩略图（同步版本，保持向后兼容）
    
    参数:
        book: 书籍对象
        page_number: 页码
        thumbnail_width: 缩略图宽度
    返回:
        thumbnail_path: 缩略图路径
    """
    # 确保cache/thumbnails目录存在
    cache_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "cache", "thumbnails", book.id)
    os.makedirs(cache_dir, exist_ok=True)
    
    # 缩略图路径
    thumbnail_filename = f"{page_number}_{thumbnail_width}.jpg"
    thumbnail_path = os.path.join(cache_dir, thumbnail_filename)
    
    # 如果缩略图已存在，直接返回
    if os.path.exists(thumbnail_path):
        return thumbnail_path
    
    try:
        if book.type == "pdf_book":
            # 处理PDF书籍
            from utils.pdf_utils import get_pdf_page_svg_sync
            
            # 获取PDF页面SVG（会自动处理缓存）
            pdf_page_svg = get_pdf_page_svg_sync(book.path, book.id, page_number)
            
            # 处理图片
            return process_image_to_thumbnail(pdf_page_svg, thumbnail_path, thumbnail_width)
        
        elif book.type == "image_book":
            # 处理图片包书籍
            if hasattr(book, "pages") and book.pages:
                if 0 < page_number <= len(book.pages):
                    page_path = book.pages[page_number - 1]
                    if os.path.exists(page_path):
                        return process_image_to_thumbnail(page_path, thumbnail_path, thumbnail_width)
                    else:
                        raise ValueError(f"Image path does not exist: {page_path}")
                else:
                    raise ValueError(f"Page number {page_number} out of range")
            else:
                raise ValueError("Image book has no pages")
        
        else:
            raise ValueError(f"Unsupported book type: {book.type}")
    
    except Exception as e:
        print(f"Error generating thumbnail for book {book.id}, page {page_number}: {e}")
        raise

# 生成书籍页码缩略图（通用版本，根据配置自动选择同步或异步）
async def generate_thumbnail(book, page_number: int, thumbnail_width: int) -> str:
    """
    生成书籍指定页码的缩略图（通用版本，根据配置自动选择同步或异步）
    
    参数:
        book: 书籍对象
        page_number: 页码
        thumbnail_width: 缩略图宽度
    返回:
        thumbnail_path: 缩略图路径
    """
    use_process_pool = get("use_process_pool", False)
    
    if use_process_pool:
        return await generate_thumbnail_async(book, page_number, thumbnail_width)
    else:
        return generate_thumbnail_sync(book, page_number, thumbnail_width)

# 主函数，用于测试
if __name__ == "__main__":
    # 测试生成封面
    from database.db import get_db
    db = get_db()
    test_book_id = "eddbada9-7ca0-5d62-a905-ee468e480f55"  # 真实存在的PDF书籍ID
    test_book = db.get_book_by_id(test_book_id)
    if test_book:
        try:
            import asyncio
            cover_path = asyncio.run(generate_cover(test_book))
            print(f"Cover generated successfully: {cover_path}")
        except Exception as e:
            print(f"Error: {e}")
    
    # 测试生成缩略图
    if test_book:
        try:
            import asyncio
            thumbnail_path = asyncio.run(generate_thumbnail(test_book, 1, 200))
            print(f"Thumbnail generated successfully: {thumbnail_path}")
        except Exception as e:
            print(f"Thumbnail error: {e}")
