from fastapi import APIRouter, HTTPException, Depends, Query
from fastapi.responses import FileResponse
from typing import List, Dict, Any
import os
from models.book import Book, BookCreate, BookUpdate, BookResponse
from database.db import get_db, Database
from utils.cache_utils import get_cache_directory

# 检查缩略图是否存在
def check_thumbnail_exists(book_id: str, page: int, width: int) -> str:
    """
    检查指定书籍页码的缩略图是否存在
    
    参数:
        book_id: 书籍ID
        page: 页码
        width: 缩略图宽度
    返回:
        缩略图路径（如果存在），否则返回空字符串
    """
    # 确保cache/thumbnails目录存在
    cache_dir = get_cache_directory()
    thumbnails_dir = cache_dir / "thumbnails" / book_id
    thumbnails_dir.mkdir(parents=True, exist_ok=True)
    
    # 缩略图路径
    thumbnail_filename = f"{page}_{width}.jpg"
    thumbnail_path = str(thumbnails_dir / thumbnail_filename)
    
    # 检查缩略图是否存在
    if os.path.exists(thumbnail_path):
        return thumbnail_path
    return ""

router = APIRouter()

# 分页响应模型
def create_paginated_response(items: List[BookResponse], total: int, page: int, page_size: int) -> Dict[str, Any]:
    """创建分页响应"""
    return {
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
        "total_pages": (total + page_size - 1) // page_size
    }

# 获取所有收藏的书籍
@router.get("/favorite/list")
async def get_favorite_books(
    page: int = Query(1, ge=1, description="页码"),
    page_size: int = Query(10, ge=1, le=100, description="每页数量"),
    sort: str = Query("desc", description="排序方式：asc（升序）或 desc（降序）"),
    db: Database = Depends(get_db)
):
    """获取所有收藏的书籍"""
    books = db.get_all_books()
    
    # 筛选收藏的书籍
    favorite_books = [book for book in books if book.is_favorite]
    
    # 根据创建时间排序
    favorite_books.sort(key=lambda x: x.created_at or "", reverse=(sort == "desc"))
    
    total = len(favorite_books)
    
    # 计算分页索引
    start = (page - 1) * page_size
    end = start + page_size
    
    # 分页数据
    paginated_books = favorite_books[start:end]
    book_responses = [BookResponse(**book.to_dict()) for book in paginated_books]
    
    # 返回分页响应
    return create_paginated_response(book_responses, total, page, page_size)

# 获取所有书籍（支持分页和排序）
@router.get("")
async def get_books(
    page: int = Query(1, ge=1, description="页码"),
    page_size: int = Query(10, ge=1, le=100, description="每页数量"),
    sort: str = Query("desc", description="排序方式：asc（升序）或 desc（降序）"),
    db: Database = Depends(get_db)
):
    books = db.get_all_books()
    
    # 根据创建时间排序
    books.sort(key=lambda x: x.created_at or "", reverse=(sort == "desc"))
    
    total = len(books)
    
    # 计算分页索引
    start = (page - 1) * page_size
    end = start + page_size
    
    # 分页数据
    paginated_books = books[start:end]
    book_responses = [BookResponse(**book.to_dict()) for book in paginated_books]
    
    # 返回分页响应
    return create_paginated_response(book_responses, total, page, page_size)

# 获取需要优化的书籍（optimization_strategy >= 2）
@router.get("/optimization-needed")
async def get_optimization_needed_books(
    page: int = Query(1, ge=1, description="页码"),
    page_size: int = Query(10, ge=1, le=100, description="每页数量"),
    sort: str = Query("desc", description="排序方式：asc（升序）或 desc（降序）"),
    db: Database = Depends(get_db)
):
    # 获取所有书籍
    all_books = db.get_all_books()
    
    # 筛选 optimization_strategy >= 2 的书籍
    optimized_books = [book for book in all_books if book.optimization_strategy >= 2]
    
    # 根据创建时间排序
    optimized_books.sort(key=lambda x: x.created_at or "", reverse=(sort == "desc"))
    
    total = len(optimized_books)
    
    # 计算分页索引
    start = (page - 1) * page_size
    end = start + page_size
    
    # 分页数据
    paginated_books = optimized_books[start:end]
    book_responses = [BookResponse(**book.to_dict()) for book in paginated_books]
    
    # 返回分页响应
    return create_paginated_response(book_responses, total, page, page_size)

# 返回PDF文件
@router.get("/pdf/{book_id}")
async def get_pdf_book(book_id: str, db: Database = Depends(get_db)):
    # 获取书籍信息
    book = db.get_book_by_id(book_id)
    if not book:
        raise HTTPException(status_code=404, detail="Book not found")
    
    # 检查书籍类型是否为PDF
    if book.type != "pdf_book":
        raise HTTPException(status_code=400, detail="This book is not a PDF book")
    
    # 检查文件是否存在
    if not os.path.exists(book.path):
        raise HTTPException(status_code=404, detail="PDF file not found")
    
    # 返回PDF文件
    return FileResponse(
        path=book.path,
        media_type="application/pdf",
        filename=os.path.basename(book.path)
    )

# 返回书籍封面
@router.get("/covers/{book_id}")
async def get_book_cover(book_id: str, db: Database = Depends(get_db)):
    # 获取书籍信息
    book = db.get_book_by_id(book_id)
    if not book:
        raise HTTPException(status_code=404, detail="Book not found")
    
    # 导入生成封面的函数
    from utils.image_utils import generate_cover
    
    try:
        # 生成封面
        cover_path = await generate_cover(book)
        
        # 检查封面是否存在
        if not os.path.exists(cover_path):
            raise HTTPException(status_code=500, detail="Failed to generate cover")
        
        # 返回封面图片
        return FileResponse(
            path=cover_path,
            media_type="image/jpeg"
        )
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

# 把书加入收藏
@router.post("/{book_id}/favorite")
async def add_book_to_favorite(book_id: str, db: Database = Depends(get_db)):
    """把书加入收藏"""
    # 获取书籍信息
    book = db.get_book_by_id(book_id)
    if not book:
        raise HTTPException(status_code=404, detail="Book not found")
    
    # 打印当前收藏状态
    print(f"当前收藏状态: {getattr(book, 'is_favorite', False)}")
    
    # 更新数据库中的收藏状态
    result = db.update_book_favorite_status(book_id, True)
    print(f"更新结果: {result}")
    
    # 重建缓存
    db.clear_cache()
    db.build_cache()
    
    # 再次获取书籍信息，检查收藏状态
    updated_book = db.get_book_by_id(book_id)
    print(f"更新后收藏状态: {getattr(updated_book, 'is_favorite', False)}")
    
    return {"message": "Book added to favorite successfully", "book_id": book_id, "is_favorite": True}

# 把书从收藏删除
@router.delete("/{book_id}/favorite")
async def remove_book_from_favorite(book_id: str, db: Database = Depends(get_db)):
    """把书从收藏删除"""
    # 获取书籍信息
    book = db.get_book_by_id(book_id)
    if not book:
        raise HTTPException(status_code=404, detail="Book not found")
    
    # 更新数据库中的收藏状态
    db.update_book_favorite_status(book_id, False)
    
    # 重建缓存
    db.clear_cache()
    db.build_cache()
    
    return {"message": "Book removed from favorite successfully", "book_id": book_id, "is_favorite": False}

# 返回书籍指定页码的图片
@router.get("/{book_id}/{page}")
async def get_book_page(
    book_id: str, 
    page: int, 
    width: int = Query(None, ge=100, le=2000, description="缩略图宽度，不提供则返回原图"),
    realsize: bool = Query(False, description="是否返回原图，默认false"),
    db: Database = Depends(get_db)
):
    # 获取书籍信息
    book = db.get_book_by_id(book_id)
    if not book:
        raise HTTPException(status_code=404, detail="Book not found")
    
    # 处理带宽度参数的缩略图请求
    if width:
        # 检查缩略图是否存在
        existing_thumbnail = check_thumbnail_exists(book.id, page, width)
        if existing_thumbnail:
            return FileResponse(
                path=existing_thumbnail,
                media_type="image/jpeg"
            )
        
        # 导入生成缩略图的函数
        from utils.image_utils import generate_thumbnail
        
        try:
            # 生成缩略图（此方法对PDF和图片包都通用）
            thumbnail_path = await generate_thumbnail(book, page, width)
            
            # 检查缩略图是否存在
            if not os.path.exists(thumbnail_path):
                raise HTTPException(status_code=500, detail="Failed to generate thumbnail")
            
            # 返回缩略图
            return FileResponse(
                path=thumbnail_path,
                media_type="image/jpeg"
            )
        except Exception as e:
            raise HTTPException(status_code=500, detail=str(e))
    
    # 处理不带宽度参数的请求
    if book.type == "image_book":
        # 检查页码是否有效
        if page < 1 or page > len(book.pages):
            raise HTTPException(status_code=400, detail=f"Invalid page number. Valid range: 1-{len(book.pages)}")
        
        # 获取图片路径
        image_path = book.pages[page - 1]
        
        # 检查图片文件是否存在
        if not os.path.exists(image_path):
            raise HTTPException(status_code=404, detail="Page image not found")
        
        # 检查是否需要压缩
        if not realsize and hasattr(book, 'optimization_strategy') and book.optimization_strategy >= 2:
            # 导入生成缩略图的函数和配置
            from utils.image_utils import generate_thumbnail
            from config.config import compressed_width
            
            # 检查是否已经生成过压缩图片
            existing_compressed = check_thumbnail_exists(book.id, page, compressed_width)
            if existing_compressed:
                return FileResponse(
                    path=existing_compressed,
                    media_type="image/jpeg"
                )
            
            try:
                # 生成压缩后的图片（使用compressed_width作为宽度）
                compressed_path = await generate_thumbnail(book, page, compressed_width)
                
                # 检查压缩后的图片是否存在
                if not os.path.exists(compressed_path):
                    raise HTTPException(status_code=500, detail="Failed to generate compressed image")
                
                # 返回压缩后的图片
                return FileResponse(
                    path=compressed_path,
                    media_type="image/jpeg"
                )
            except Exception as e:
                raise HTTPException(status_code=500, detail=str(e))
        else:
            # 直接返回原图
            # 确定图片的MIME类型
            import mimetypes
            mime_type, _ = mimetypes.guess_type(image_path)
            if not mime_type:
                mime_type = "image/jpeg"  # 默认类型
            
            # 直接返回原图
            return FileResponse(
                path=image_path,
                media_type=mime_type
            )
    elif book.type == "pdf_book":
        # PDF书籍请请求SVG格式
        raise HTTPException(status_code=400, detail="PDF books should request SVG format! Please use /api/books/svg/{book_id}/{page}")
    else:
        raise HTTPException(status_code=400, detail="This book type does not support page images")

# 根据 ID 获取书籍
@router.get("/{book_id}", response_model=BookResponse)
async def get_book(book_id: str, db: Database = Depends(get_db)):
    book = db.get_book_by_id(book_id)
    if not book:
        raise HTTPException(status_code=404, detail="Book not found")
    return BookResponse(**book.to_dict())

# 返回PDF书籍指定页面的SVG
@router.get("/svg/{book_id}/{page}")
async def get_book_page_svg(
    book_id: str, 
    page: int, 
    db: Database = Depends(get_db)
):
    """
    获取PDF书籍指定页面的SVG
    
    Args:
        book_id: 书籍ID
        page: 页码（从1开始）
        db: 数据库依赖
        
    Returns:
        FileResponse: SVG文件响应
    """
    try:
        # 获取书籍信息
        book = db.get_book_by_id(book_id)
        if not book:
            raise HTTPException(status_code=404, detail="Book not found")
        
        # 检查是否是PDF书籍
        if book.type != "pdf_book":
            raise HTTPException(status_code=400, detail="Only PDF books support SVG conversion")
        
        # 检查页码是否有效
        if page < 1 or page > book.page_count:
            raise HTTPException(status_code=400, detail=f"Invalid page number. Valid range: 1-{book.page_count}")
        
        # 获取PDF文件路径
        pdf_path = book.path
        if not os.path.exists(pdf_path):
            raise HTTPException(status_code=404, detail="PDF file not found")
        
        # 导入PDF工具函数
        from utils.pdf_utils import get_pdf_page_svg
        
        # 获取SVG文件路径
        svg_path = await get_pdf_page_svg(pdf_path, book_id, page)
        
        # 检查SVG文件是否存在
        if not os.path.exists(svg_path):
            raise HTTPException(status_code=500, detail="Failed to generate SVG")
        
        # 返回SVG文件
        return FileResponse(
            path=svg_path,
            media_type="image/svg+xml"
        )
        
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))
    except Exception as e:
        print(f"Error getting PDF page SVG: {e}")
        raise HTTPException(status_code=500, detail="Internal server error")
