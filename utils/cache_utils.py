import os
import shutil
from pathlib import Path

def clear_book_cache(book_id: str) -> bool:
    """根据书籍ID删除相关缓存
    
    Args:
        book_id: 书籍ID
        
    Returns:
        bool: 删除是否成功
    """
    try:
        # 构建缓存目录路径
        project_root = Path(__file__).parent.parent
        
        # 1. 删除封面缓存
        cover_dir = project_root / "cache" / "covers"
        cover_path = cover_dir / f"{book_id}.jpg"
        if cover_path.exists():
            cover_path.unlink()
            print(f"Deleted cover cache for book: {book_id}")
        
        # 2. 删除缩略图缓存
        thumbnails_dir = project_root / "cache" / "thumbnails" / book_id
        if thumbnails_dir.exists():
            shutil.rmtree(thumbnails_dir)
            print(f"Deleted thumbnail cache for book: {book_id}")
        
        # 3. 删除PDF SVG缓存
        pdf_cache_dir = project_root / "cache" / "pdf" / book_id
        if pdf_cache_dir.exists():
            shutil.rmtree(pdf_cache_dir)
            print(f"Deleted PDF SVG cache for book: {book_id}")
        
        return True
    except Exception as e:
        print(f"Error clearing cache for book {book_id}: {e}")
        return False

def clear_all_cache() -> bool:
    """清空所有缓存
    
    Returns:
        bool: 删除是否成功
    """
    try:
        # 构建缓存目录路径
        project_root = Path(__file__).parent.parent
        cache_dir = project_root / "cache"
        
        # 删除所有缓存目录
        if cache_dir.exists():
            for item in cache_dir.iterdir():
                if item.is_dir():
                    shutil.rmtree(item)
                    print(f"Deleted cache directory: {item.name}")
        
        return True
    except Exception as e:
        print(f"Error clearing all cache: {e}")
        return False
