from fastapi import APIRouter, HTTPException, Depends
from typing import List
from models.category import Category, CategoryCreate, CategoryUpdate, CategoryResponse
from database.db import get_db, Database

router = APIRouter()

# 获取所有分类
@router.get("")
async def get_categories(db: Database = Depends(get_db)):
    categories = db.get_all_categories()
    
    # 处理分类数据，移除书籍中的pages字段
    def process_category(category):
        category_dict = category.to_dict()
        # 处理书籍，移除pages字段
        processed_books = []
        for book in category.books:
            # 处理Book对象
            if hasattr(book, 'to_dict'):
                book_dict = book.to_dict()
                # 移除pages字段
                if 'pages' in book_dict:
                    del book_dict['pages']
                processed_books.append(book_dict)
        category_dict['books'] = processed_books
        
        # 递归处理子分类
        processed_sub_categories = []
        for sub_cat in category.sub_categories:
            processed_sub_categories.append(process_category(sub_cat))
        category_dict['sub_categories'] = processed_sub_categories
        
        return category_dict
    
    # 只返回根分类（第一个分类），因为根分类已经包含了所有子分类
    if categories:
        # 处理根分类
        root_category = process_category(categories[0])
        return [root_category]
    
    return []

# 根据 ID 获取分类
@router.get("/{category_id}", response_model=CategoryResponse)
async def get_category(category_id: str, db: Database = Depends(get_db)):
    category = db.get_category_by_id(category_id)
    if not category:
        raise HTTPException(status_code=404, detail="Category not found")
    
    # 处理分类数据，移除书籍中的pages字段
    category_dict = category.to_dict()
    
    # 处理书籍，移除pages字段
    processed_books = []
    for book in category.books:
        # 处理Book对象
        if hasattr(book, 'to_dict'):
            book_dict = book.to_dict()
            # 移除pages字段
            if 'pages' in book_dict:
                del book_dict['pages']
            processed_books.append(book_dict)
    category_dict['books'] = processed_books
    
    # 递归处理子分类
    def process_sub_categories(sub_categories):
        processed = []
        for sub_cat in sub_categories:
            # 处理Category对象
            sub_cat_dict = sub_cat.to_dict()
            # 处理子分类中的书籍
            sub_cat_books = []
            for book in sub_cat.books:
                if hasattr(book, 'to_dict'):
                    book_dict = book.to_dict()
                    if 'pages' in book_dict:
                        del book_dict['pages']
                    sub_cat_books.append(book_dict)
            sub_cat_dict['books'] = sub_cat_books
            
            # 递归处理子分类的子分类
            if sub_cat.sub_categories:
                sub_cat_dict['sub_categories'] = process_sub_categories(sub_cat.sub_categories)
            processed.append(sub_cat_dict)
        return processed
    
    if category.sub_categories:
        category_dict['sub_categories'] = process_sub_categories(category.sub_categories)
    
    return CategoryResponse(**category_dict)

# 获取分类下的所有书籍（包括子分类）
@router.get("/{category_id}/books")
async def get_category_books(category_id: str, db: Database = Depends(get_db)):
    """
    获取指定分类及其子分类下的所有书籍
    """
    category = db.get_category_by_id(category_id)
    if not category:
        raise HTTPException(status_code=404, detail="Category not found")
    
    # 递归获取所有书籍
    def get_books_recursive(category_obj):
        books = []
        # 添加当前分类的书籍
        if category_obj.books:
            for book in category_obj.books:
                # 处理Book对象
                if hasattr(book, 'to_dict'):
                    book_dict = book.to_dict()
                    # 移除pages字段
                    if 'pages' in book_dict:
                        del book_dict['pages']
                    books.append(book_dict)
        
        # 递归获取子分类的书籍
        if category_obj.sub_categories:
            for sub_cat in category_obj.sub_categories:
                # 直接处理Category对象
                books.extend(get_books_recursive(sub_cat))
        
        return books
    
    # 获取所有书籍
    all_books = get_books_recursive(category)
    
    # 去重（如果有重复的书籍）
    unique_books = []
    seen_ids = set()
    for book in all_books:
        if book['id'] not in seen_ids:
            seen_ids.add(book['id'])
            unique_books.append(book)
    
    return unique_books
