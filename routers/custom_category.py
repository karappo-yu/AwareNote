from fastapi import APIRouter, HTTPException, Depends, Query
from typing import List
from database.db import get_db, Database
from models.category import CustomCategoryBase, CustomCategoryCreate, CustomCategoryUpdate, CustomCategoryResponse
from models.book import BookResponse

router = APIRouter()

# 分页响应模型
def create_paginated_response(items: List, total: int, page: int, page_size: int):
    """创建分页响应"""
    return {
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
        "total_pages": (total + page_size - 1) // page_size
    }

# 获取所有自定义分类（支持分页）
@router.get("")
async def get_custom_categories(
    page: int = Query(1, ge=1, description="页码"),
    page_size: int = Query(10, ge=1, le=100, description="每页数量"),
    db: Database = Depends(get_db)
):
    custom_categories = db.get_all_custom_categories()
    total = len(custom_categories)
    
    # 计算分页索引
    start = (page - 1) * page_size
    end = start + page_size
    
    # 分页数据
    paginated_custom_categories = custom_categories[start:end]
    custom_category_responses = [CustomCategoryResponse(**cc) for cc in paginated_custom_categories]
    
    # 返回分页响应
    return create_paginated_response(custom_category_responses, total, page, page_size)

# 根据 ID 获取自定义分类
@router.get("/{custom_category_id}")
async def get_custom_category(custom_category_id: str, db: Database = Depends(get_db)):
    custom_category = db.get_custom_category_by_id(custom_category_id)
    if not custom_category:
        raise HTTPException(status_code=404, detail="Custom category not found")
    return CustomCategoryResponse(**custom_category)

# 创建自定义分类
@router.post("")
async def create_custom_category(custom_category_data: CustomCategoryCreate, db: Database = Depends(get_db)):
    try:
        # 生成 ID（如果未提供）
        import uuid
        custom_category_dict = custom_category_data.dict()
        if not custom_category_dict.get('id'):
            custom_category_dict['id'] = str(uuid.uuid4())
        
        # 设置创建和更新时间
        from datetime import datetime
        now = datetime.now().isoformat()
        custom_category_dict['created_at'] = now
        custom_category_dict['updated_at'] = now
        
        # 设置默认值
        if 'book_count' not in custom_category_dict:
            custom_category_dict['book_count'] = 0
        
        db.add_custom_category(custom_category_dict)
        return CustomCategoryResponse(**custom_category_dict)
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

# 更新自定义分类
@router.put("/{custom_category_id}")
async def update_custom_category(custom_category_id: str, custom_category_data: CustomCategoryUpdate, db: Database = Depends(get_db)):
    try:
        # 检查自定义分类是否存在
        existing_custom_category = db.get_custom_category_by_id(custom_category_id)
        if not existing_custom_category:
            raise HTTPException(status_code=404, detail="Custom category not found")
        
        # 设置 ID 和更新时间
        custom_category_dict = custom_category_data.dict(exclude_unset=True)
        custom_category_dict['id'] = custom_category_id
        from datetime import datetime
        custom_category_dict['updated_at'] = datetime.now().isoformat()
        
        # 合并现有数据
        for key, value in existing_custom_category.items():
            if key not in custom_category_dict:
                custom_category_dict[key] = value
        
        db.update_custom_category(custom_category_dict)
        return CustomCategoryResponse(**custom_category_dict)
    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

# 删除自定义分类
@router.delete("/{custom_category_id}")
async def delete_custom_category(custom_category_id: str, db: Database = Depends(get_db)):
    try:
        # 检查自定义分类是否存在
        existing_custom_category = db.get_custom_category_by_id(custom_category_id)
        if not existing_custom_category:
            raise HTTPException(status_code=404, detail="Custom category not found")
        
        db.delete_custom_category(custom_category_id)
        return {"message": "Custom category deleted successfully"}
    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

# 添加书籍到自定义分类
@router.post("/{custom_category_id}/books/{book_id}")
async def add_book_to_custom_category(custom_category_id: str, book_id: str, db: Database = Depends(get_db)):
    try:
        db.add_book_to_custom_category(book_id, custom_category_id)
        return {"message": "Book added to custom category successfully"}
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

# 从自定义分类中移除书籍
@router.delete("/{custom_category_id}/books/{book_id}")
async def remove_book_from_custom_category(custom_category_id: str, book_id: str, db: Database = Depends(get_db)):
    try:
        db.remove_book_from_custom_category(book_id, custom_category_id)
        return {"message": "Book removed from custom category successfully"}
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

# 获取自定义分类中的书籍（支持分页）
@router.get("/{custom_category_id}/books")
async def get_books_in_custom_category(
    custom_category_id: str,
    page: int = Query(1, ge=1, description="页码"),
    page_size: int = Query(10, ge=1, le=100, description="每页数量"),
    db: Database = Depends(get_db)
):
    try:
        # 检查自定义分类是否存在
        existing_custom_category = db.get_custom_category_by_id(custom_category_id)
        if not existing_custom_category:
            raise HTTPException(status_code=404, detail="Custom category not found")
        
        books = db.get_books_in_custom_category(custom_category_id)
        total = len(books)
        
        # 计算分页索引
        start = (page - 1) * page_size
        end = start + page_size
        
        # 分页数据
        paginated_books = books[start:end]
        book_responses = [BookResponse(**book.to_dict()) for book in paginated_books]
        
        # 返回分页响应
        return create_paginated_response(book_responses, total, page, page_size)
    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

# 获取书籍所属的自定义分类
@router.get("/books/{book_id}")
async def get_custom_categories_for_book(book_id: str, db: Database = Depends(get_db)):
    try:
        custom_categories = db.get_custom_categories_for_book(book_id)
        return [CustomCategoryResponse(**cc) for cc in custom_categories]
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))
