import os
import sys
import fitz  # PyMuPDF
import time
import asyncio

# 添加项目根目录到导入路径
sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))

# 导入配置模块
from config.config import get

# 导入全局进程池管理模块
from utils.pool_manager import get_thread_pool, get_processing_tasks, get_processing_lock

# 正在处理的任务集合，用于避免重复任务
_processing_tasks = get_processing_tasks()
_processing_lock = get_processing_lock()

def get_pdf_cache_dir(book_id):
    """
    获取PDF缓存目录
    """
    root_dir = os.path.dirname(os.path.dirname(__file__))
    cache_dir = os.path.join(root_dir, "cache", "pdf", book_id)
    os.makedirs(cache_dir, exist_ok=True)
    return cache_dir

# 解耦的处理函数 - 将PDF页面转换为SVG（纯同步函数）
def process_pdf_page_to_svg(pdf_path, book_id, page_num):
    """
    将PDF指定页码转换为SVG并缓存
    
    Args:
        pdf_path: PDF文件路径
        book_id: 书籍ID
        page_num: 页码（从1开始）
        
    Returns:
        str: SVG文件路径
    """
    cache_dir = get_pdf_cache_dir(book_id)
    svg_path = os.path.join(cache_dir, f"{page_num}.svg")
    
    # 检查是否已经缓存
    if os.path.exists(svg_path):
        return svg_path
    
    # 处理PDF页面为SVG
    try:
        with fitz.open(pdf_path) as pdf:
            if page_num < 1 or page_num > pdf.page_count:
                raise ValueError(f"Invalid page number {page_num}")
            
            # 获取指定页面
            page = pdf[page_num - 1]
            
            # 转换为SVG
            svg_content = page.get_svg_image()
            
            # 保存SVG文件
            with open(svg_path, "w", encoding="utf-8") as f:
                f.write(svg_content)
        
        return svg_path
        
    except Exception as e:
        print(f"Error converting PDF page to SVG: {e}")
        raise

# 获取PDF指定页码的SVG（同步版本）
def get_pdf_page_svg_sync(pdf_path, book_id, page_num):
    """
    获取PDF指定页码的SVG
    
    Args:
        pdf_path: PDF文件路径
        book_id: 书籍ID
        page_num: 页码（从1开始）
        
    Returns:
        str: SVG文件路径
    """
    return process_pdf_page_to_svg(pdf_path, book_id, page_num)

# 获取PDF指定页码的SVG（通用版本，根据配置自动选择同步或异步）
async def get_pdf_page_svg(pdf_path, book_id, page_num):
    """
    获取PDF指定页码的SVG（通用版本，根据配置自动选择同步或异步）
    
    Args:
        pdf_path: PDF文件路径
        book_id: 书籍ID
        page_num: 页码（从1开始）
        
    Returns:
        str: SVG文件路径
    """
    use_process_pool = get("use_process_pool", False)
    
    if use_process_pool:
        return await get_pdf_page_svg_async(pdf_path, book_id, page_num)
    else:
        return get_pdf_page_svg_sync(pdf_path, book_id, page_num)

# 获取PDF指定页码的SVG（异步版本）
async def get_pdf_page_svg_async(pdf_path, book_id, page_num):
    """
    异步获取PDF指定页码的SVG
    
    Args:
        pdf_path: PDF文件路径
        book_id: 书籍ID
        page_num: 页码（从1开始）
        
    Returns:
        str: SVG文件路径
    """
    # 检查是否已经缓存
    cache_dir = get_pdf_cache_dir(book_id)
    svg_path = os.path.join(cache_dir, f"{page_num}.svg")
    
    # 如果已缓存，直接返回
    if os.path.exists(svg_path):
        return svg_path
    
    # 创建任务标识
    task_id = f"get_pdf_page_svg:{book_id}:{page_num}"
    
    # 检查是否已有相同任务在处理中
    with _processing_lock:
        if task_id in _processing_tasks:
            # 等待现有任务完成
            while task_id in _processing_tasks:
                await asyncio.sleep(0.1)
            # 再次检查缓存
            if os.path.exists(svg_path):
                return svg_path
        _processing_tasks.add(task_id)
    
    try:
        from utils.pool_manager import submit_task_to_pool
        
        # 使用新的submit_task_to_pool函数提交任务
        svg_path = await submit_task_to_pool(
            process_pdf_page_to_svg,
            pdf_path,
            book_id,
            page_num
        )
        
        return svg_path
        
    except Exception as e:
        print(f"Error converting PDF page to SVG asynchronously: {e}")
        raise
    
    finally:
        # 从处理集合中移除
        with _processing_lock:
            _processing_tasks.discard(task_id)
