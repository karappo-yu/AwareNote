#!/usr/bin/env python3
"""
测试缓存功能
"""

import sys
from pathlib import Path

# 添加项目根目录到导入路径
sys.path.insert(0, str(Path(__file__).parent))

from utils.cache_utils import get_cache_directory, clear_all_cache, clear_book_cache

def test_cache_directory():
    """测试缓存目录获取"""
    print("=== 测试缓存目录获取 ===")
    cache_dir = get_cache_directory()
    print(f"缓存目录: {cache_dir}")
    print(f"目录是否存在: {cache_dir.exists()}")
    print()

def test_clear_all_cache():
    """测试清空所有缓存"""
    print("=== 测试清空所有缓存 ===")
    result = clear_all_cache()
    print(f"清空所有缓存结果: {result}")
    print()

def test_clear_book_cache():
    """测试清空指定书籍缓存"""
    print("=== 测试清空指定书籍缓存 ===")
    test_book_id = "test_book_123"
    result = clear_book_cache(test_book_id)
    print(f"清空书籍缓存结果: {result}")
    print()

if __name__ == "__main__":
    print("开始测试缓存功能...\n")
    
    test_cache_directory()
    test_clear_all_cache()
    test_clear_book_cache()
    
    print("测试完成!")
