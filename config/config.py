import json
import os
import sys
from pathlib import Path

# 处理PyInstaller打包后的路径
def get_config_dir():
    if getattr(sys, 'frozen', False):
        # 打包后的环境
        executable_dir = os.path.dirname(sys.executable)
        # 在macOS应用包中，资源文件位于Contents/Resources目录
        return os.path.join(executable_dir, '..', 'Resources', 'config')
    else:
        # 开发环境
        return os.path.dirname(__file__)

# 加载配置文件
def load_config():
    config_dir = get_config_dir()
    config_path = os.path.join(config_dir, "setting.json")
    with open(config_path, "r", encoding="utf-8") as f:
        return json.load(f)

# 加载配置数据
_config_data = load_config()

# 配置项
root_path = _config_data.get("root_path")
ignored_file_types = _config_data.get("ignored_file_types", [])
cover_width = _config_data.get("cover_width", 1200)
scan_strategy_max_width = _config_data.get("scan_strategy_max_width", 2500)
scan_strategy_max_length = _config_data.get("scan_strategy_max_length", 2500)
scan_strategy_max_pixel_area = _config_data.get("scan_strategy_max_pixel_area", 5000000)
compressed_width = _config_data.get("compressedWidth", 1920)
image_exts = _config_data.get("image_exts", [".jpg", ".jpeg", ".png", ".gif", ".bmp", ".tif", ".tiff", ".webp", ".avif", ".heic", ".svg"])
auto_scan_on_startup = _config_data.get("auto_scan_on_startup", False)



# 获取配置项的函数
def get(key, default=None):
    return _config_data.get(key, default)

# 更新配置项的函数
def update_config(new_config):
    global _config_data
    
    # 更新内存中的配置
    _config_data.update(new_config)
    
    # 写回配置文件
    config_dir = get_config_dir()
    config_path = os.path.join(config_dir, "setting.json")
    with open(config_path, "w", encoding="utf-8") as f:
        json.dump(_config_data, f, indent=4, ensure_ascii=False)
    
    # 更新全局变量
    global root_path, ignored_file_types, cover_width, scan_strategy_max_width
    global scan_strategy_max_length, scan_strategy_max_pixel_area, compressed_width
    global image_exts, auto_scan_on_startup
    
    root_path = _config_data.get("root_path")
    ignored_file_types = _config_data.get("ignored_file_types", [])
    cover_width = _config_data.get("cover_width", 1200)
    scan_strategy_max_width = _config_data.get("scan_strategy_max_width", 2500)
    scan_strategy_max_length = _config_data.get("scan_strategy_max_length", 2500)
    scan_strategy_max_pixel_area = _config_data.get("scan_strategy_max_pixel_area", 5000000)
    compressed_width = _config_data.get("compressedWidth", 1920)
    image_exts = _config_data.get("image_exts", [".jpg", ".jpeg", ".png", ".gif", ".bmp", ".tif", ".tiff", ".webp", ".avif", ".heic", ".svg"])
    auto_scan_on_startup = _config_data.get("auto_scan_on_startup", False)
