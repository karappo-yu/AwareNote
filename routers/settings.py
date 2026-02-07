from fastapi import APIRouter, HTTPException
import os
import shutil

router = APIRouter()

def get_directory_size(directory):
    """
    计算目录大小（字节）
    """
    total_size = 0
    for dirpath, dirnames, filenames in os.walk(directory):
        for filename in filenames:
            filepath = os.path.join(dirpath, filename)
            if os.path.exists(filepath):
                try:
                    total_size += os.path.getsize(filepath)
                except Exception:
                    pass
    return total_size

def clear_cache():
    """
    清空缓存并返回释放的空间大小（MB）
    """
    # 获取项目根目录
    root_dir = os.path.dirname(os.path.dirname(__file__))
    cache_dir = os.path.join(root_dir, "cache")
    
    # 检查cache目录是否存在
    if not os.path.exists(cache_dir):
        return 0.0
    
    # 计算清理前的缓存大小
    before_size = get_directory_size(cache_dir)
    
    try:
        # 递归删除cache目录及其内容
        shutil.rmtree(cache_dir)
        
        # 重新创建空的cache目录
        os.makedirs(cache_dir, exist_ok=True)
        
        # 计算释放的空间（转换为MB）
        space_freed_mb = before_size / (1024 * 1024)
        return round(space_freed_mb, 1)
        
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to clear cache: {str(e)}")

# 清空系统缓存
@router.delete("/cache/clear")
async def clear_system_cache():
    """
    清空系统缓存
    返回释放的空间大小（MB）
    """
    space_freed_mb = clear_cache()
    return {"space_freed_mb": space_freed_mb}

# 获取系统配置和运行状态
@router.get("/config")
async def get_system_config():
    """
    获取系统配置和运行状态
    返回系统统计信息和配置
    """
    from database.db import get_db
    from config.config import get
    
    # 获取数据库实例
    db = get_db()
    
    # 获取总书籍数量
    total_books = len(db.get_all_books())
    
    # 计算缓存大小
    root_dir = os.path.dirname(os.path.dirname(__file__))
    cache_dir = os.path.join(root_dir, "cache")
    cache_size_mb = 0.0
    if os.path.exists(cache_dir):
        cache_size_mb = get_directory_size(cache_dir) / (1024 * 1024)
    cache_size_mb = round(cache_size_mb, 1)
    
    # 从配置文件中获取版本号
    version = get("version", "v1.0.0")
    
    # 服务器状态
    server_status = "healthy"
    
    # 获取系统配置
    settings = {
        "root_path": get("root_path"),
        "ignored_file_types": get("ignored_file_types", []),
        "cover_width": get("cover_width", 1200),
        "scan_strategy_max_width": get("scan_strategy_max_width", 2500),
        "scan_strategy_max_length": get("scan_strategy_max_length", 2500),
        "scan_strategy_max_pixel_area": get("scan_strategy_max_pixel_area", 5000000),
        "compressedWidth": get("compressedWidth", 1920),
        "image_exts": get("image_exts", [".jpg", ".jpeg", ".png", ".gif", ".webp"]),
        "auto_scan_on_startup": get("auto_scan_on_startup", False),
        "use_thread_pool": get("use_thread_pool", True),
        "thread_pool_max_workers": get("thread_pool_max_workers", 9),
        "thread_pool_idle_timeout": get("thread_pool_idle_timeout", 30)
    }
    
    # 构建响应
    response = {
        "stats": {
            "total_books": total_books,
            "cache_size_mb": cache_size_mb,
            "version": version,
            "server_status": server_status
        },
        "settings": settings
    }
    
    return response

# 更新系统配置
@router.put("/config")
async def update_system_config(settings: dict):
    """
    更新系统配置
    接收完整的settings对象并更新配置
    """
    from config.config import update_config, get
    
    # 验证数据类型
    required_fields = {
        "root_path": str,
        "ignored_file_types": list,
        "cover_width": int,
        "scan_strategy_max_width": int,
        "scan_strategy_max_length": int,
        "scan_strategy_max_pixel_area": int,
        "compressedWidth": int,
        "image_exts": list,
        "auto_scan_on_startup": bool
    }
    
    for field, expected_type in required_fields.items():
        if field not in settings:
            raise HTTPException(status_code=400, detail=f"Missing required field: {field}")
        if not isinstance(settings[field], expected_type):
            raise HTTPException(status_code=400, detail=f"Invalid type for field {field}: expected {expected_type.__name__}")
    
    # 验证路径是否存在
    if not os.path.exists(settings["root_path"]):
        raise HTTPException(status_code=400, detail=f"Root path does not exist: {settings['root_path']}")
    
    # 验证数值范围
    if settings["cover_width"] <= 0:
        raise HTTPException(status_code=400, detail="Cover width must be positive")
    if settings["compressedWidth"] <= 0:
        raise HTTPException(status_code=400, detail="Compressed width must be positive")
    
    # 更新配置
    try:
        update_config(settings)
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to update config: {str(e)}")
    
    # 构建响应
    response = {
        "success": True,
        "message": "Configuration updated successfully",
        "settings": settings
    }
    
    return response
