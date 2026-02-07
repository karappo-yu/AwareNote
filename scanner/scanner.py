import os
import sys
import json
from pathlib import Path
from datetime import datetime
import uuid

# 导入PyMuPDF库用于处理PDF文件
import fitz  # PyMuPDF

# 导入PIL库用于分析图片尺寸
from PIL import Image

# 添加项目根目录到 Python 路径
sys.path.append(str(Path(__file__).parent.parent))

# 导入模型
from models.book import Book
from models.category import Category

def analyze_image_book_dimensions(page_paths):
    """
    抽样分析图片包的分辨率特征，决定是否需要压缩策略。
    
    该函数采用“Header-Only”读取方式，仅读取图像元数据而不加载像素，性能极高。
    判定逻辑考虑了宽度、高度、极端比例以及总像素面积。
    """
    # --- 配置项备注 ---
    # 以下值建议在 config.py 中定义并导入：
    # IMAGE_MAX_WIDTH: 触发压缩的宽度阈值 (如 2000)
    # IMAGE_MAX_LENGTH: 触发压缩的高度阈值 (如 2500)
    # MAX_PIXEL_AREA: 触发压缩的总像素面积阈值 (如 5000000，约500万像素)
    try:
        from config.config import scan_strategy_max_width, scan_strategy_max_length, scan_strategy_max_pixel_area
    except ImportError:
        # 兜底默认值
        scan_strategy_max_width = 2000
        scan_strategy_max_length = 2500
        scan_strategy_max_pixel_area = 5000000

    if not page_paths:
        return 1, 0, 0

    count = len(page_paths)
    
    # 1. 抽样索引计算
    # 使用 set 去重处理，自动处理单张图或少量图片的情况
    # 如果只有 1 张图，indices 将只包含 [0]
    sample_indices = sorted(list(set([0, count // 2, count - 1])))
    
    total_w = 0
    total_h = 0
    valid_samples = 0

    # 2. 抽样读取尺寸
    for idx in sample_indices:
        try:
            # 关键：PIL 的 open 是懒加载，读取 size 属性不会解码整张图
            with Image.open(page_paths[idx]) as img:
                w, h = img.size
                total_w += w
                total_h += h
                valid_samples += 1
        except Exception as e:
            # 记录异常但继续处理其他样本
            print(f"警告: 无法读取图片信息 {page_paths[idx]}: {e}")
            continue

    if valid_samples == 0:
        return 1, 0, 0

    # 3. 计算平均值
    avg_w = total_w // valid_samples
    avg_h = total_h // valid_samples
    avg_area = avg_w * avg_h

    # 4. 科学判定逻辑
    # 策略 1: 保持原样 (Normal / Original)
    # 策略 2: 建议压缩 (High-Res / Optimize)
    
    # 条件 A: 宽高均超标（标准超大扫描本）
    is_standard_large = avg_w >= scan_strategy_max_width and avg_h >= scan_strategy_max_length
    
    # 条件 B: 单边极端超标（防止超长条漫或超宽全景图导致解压内存溢出）
    # 即使另一边很小，如果单边超过阈值的 2 倍，也视为需处理对象
    is_extreme_ratio = (avg_w >= scan_strategy_max_width * 2) or (avg_h >= scan_strategy_max_length * 2)
    
    # 条件 C: 总像素面积超标（针对高像素密度的单图）
    is_too_heavy = avg_area >= scan_strategy_max_pixel_area

    if is_standard_large or is_extreme_ratio or is_too_heavy:
        strategy = 2
    else:
        strategy = 1

    return strategy, avg_w, avg_h

def analyze_pdf_dimensions(pdf_path):
    """
    原子化扫描：仅获取 PDF 的原始点数尺寸，不进行策略判定。
    """
    if not os.path.exists(pdf_path):
        return 1, 0, 0

    try:
        with fitz.open(pdf_path) as doc:
            count = doc.page_count
            if count == 0:
                return 1, 0, 0

            # 抽样首页、中间、末页
            sample_indices = sorted(list(set([0, count // 2, count - 1])))
            
            total_w, total_h, valid_samples = 0, 0, 0

            for idx in sample_indices:
                try:
                    page = doc[idx]
                    total_w += int(page.rect.width)
                    total_h += int(page.rect.height)
                    valid_samples += 1
                except:
                    continue
        
        if valid_samples == 0:
            return 1, 0, 0

        avg_w = total_w // valid_samples
        avg_h = total_h // valid_samples

        # 对于 PDF，我们统一返回 strategy 1
        # 因为我们的渲染器已经具备了“自动限宽 2000”的能力，不需要在这里提前标记
        return 1, avg_w, avg_h

    except Exception as e:
        print(f"警告: 无法分析 PDF {pdf_path}: {e}")
        return 1, 0, 0

class Scanner:
    """扫描器类，用于扫描文件系统并返回分类和书籍"""
    
    # 图片扩展名集合（从配置文件加载）
    from config.config import image_exts
    IMAGE_EXTS = set(image_exts)
    
    # UUID 命名空间（用于基于路径生成稳定 ID）
    UUID_NS = uuid.NAMESPACE_URL
    
    def __init__(self):
        """初始化扫描器"""
        pass
    
    def generate_id(self, path: str) -> str:
        """基于路径生成稳定的 UUID"""
        return str(uuid.uuid5(self.UUID_NS, path))
    
    def load_settings(self):
        """加载配置文件"""
        # 使用新的 config 模块获取配置信息
        from config.config import root_path, ignored_file_types
        
        if not root_path:
            raise ValueError("配置中必须包含 'root_path' 字段")
        
        return Path(root_path), set(ext.lower() for ext in ignored_file_types)
    
    def create_category(self, dir_path: Path, parent_id: str | None) -> Category:
        """创建分类对象"""
        path_str = str(dir_path.resolve())
        category = Category(
            id=self.generate_id(path_str),
            name=dir_path.name,
            path=path_str
        )
        category.is_deleted = False
        return category
    
    def create_pdf_book(self, pdf_path: Path, category_id: str | None) -> Book:
        """创建 PDF 书籍对象"""
        path_str = str(pdf_path.resolve())
        title = pdf_path.stem
        
        # 获取 inode 和 device_id（仅 PDF 需要）
        try:
            st = pdf_path.stat()
            inode = str(st.st_ino)
            device_id = str(st.st_dev)
        except Exception:
            inode = None
            device_id = None
        
        # 获取 PDF 页面数量
        page_count = 0
        try:
            with fitz.open(path_str) as pdf:
                page_count = pdf.page_count
        except Exception as e:
            print(f"Error reading PDF page count for {path_str}: {e}")
            page_count = 0
        
        book = Book(
            id=self.generate_id(path_str),
            title=title,
            path=path_str,
            book_type="pdf_book"
        )
        book.cover_path = None  # demo 中无法提取 PDF 封面
        book.page_count = page_count  # 设置实际的页面数量
        book.pages = []
        book.inode = inode
        book.device_id = device_id
        book.is_deleted = False
        
        # 分析 PDF 尺寸并设置优化策略
        strategy, avg_w, avg_h = analyze_pdf_dimensions(path_str)
        book.optimization_strategy = strategy
        # 直接装填原始尺寸信息
        book.page_dimension_type = f"{avg_w}x{avg_h}"
        
        return book
    
    def create_image_book(self, folder_path: Path, image_paths: list[str], category_id: str | None) -> Book:
        """创建图片文件夹书籍对象"""
        path_str = str(folder_path.resolve())
        title = folder_path.name
        page_count = len(image_paths)
        cover_path = image_paths[0] if image_paths else None
        
        book = Book(
            id=self.generate_id(path_str),
            title=title,
            path=path_str,
            book_type="image_book"
        )
        book.cover_path = cover_path
        book.page_count = page_count
        book.pages = image_paths
        book.inode = None
        book.device_id = None
        book.is_deleted = False
        
        # 分析图片包尺寸并设置优化策略
        strategy, avg_w, avg_h = analyze_image_book_dimensions(image_paths)
        book.optimization_strategy = strategy
        # 直接装填原始尺寸信息
        book.page_dimension_type = f"{avg_w}x{avg_h}"
        
        return book
    
    def scan_dir(self, current_path: Path, parent_category: Category | None, ignored_exts: set[str], categories: list, books: list, category_map: dict):
        """递归扫描目录"""
        if not current_path.exists():
            print(f"路径不存在，跳过: {current_path}")
            return
        
        try:
            entries = list(current_path.iterdir())
        except PermissionError:
            print(f"权限不足，跳过: {current_path}")
            return
        
        # 排除隐藏文件/文件夹
        non_hidden = [e for e in entries if not e.name.startswith('.')]
        subdirs = [e for e in non_hidden if e.is_dir()]
        files = [e for e in non_hidden if e.is_file()]
        
        has_subdirs = bool(subdirs)
        pdf_files = [f for f in files if f.suffix.lower() == ".pdf"]
        
        # 计算有效扩展名（排除 ignored）
        effective_exts = {
            f.suffix.lower() for f in files
            if f.suffix.lower() not in ignored_exts
        }
        
        # 是否为纯图片书（无子文件夹 + 至少有图片 + 有效文件全为图片）
        is_image_book = (
            not has_subdirs and
            any(f.suffix.lower() in self.IMAGE_EXTS for f in files) and
            effective_exts.issubset(self.IMAGE_EXTS)
        )
        
        # 情况1：有子文件夹 → 一定是分类
        # 情况2：无子文件夹 + 有PDF + 不是图片书 → 视为分类（修复的bug）
        # 其他情况（纯图片书或空/无关文件）→ 不创建当前分类
        current_category = parent_category
        if has_subdirs or (pdf_files and not is_image_book):
            category = self.create_category(current_path, parent_category.id if parent_category else None)
            # 只有当不是根目录时，才将分类添加到categories列表中
            # 根目录分类由scan方法创建，不需要重复添加
            if parent_category:
                categories.append(category)
                category_map[category.id] = category
                # 将子分类添加到父分类的sub_categories列表中
                parent_category.sub_categories.append(category)
                # 更新current_category为新创建的分类
                current_category = category
            else:
                # 如果是根目录，直接使用这个分类作为current_category
                current_category = category
                category_map[category.id] = category
        
        # 处理 PDF 书本（使用确定的 current_category）
        for pdf_path in pdf_files:
            book = self.create_pdf_book(pdf_path, current_category.id if current_category else None)
            books.append(book)
            if current_category:
                current_category.books.append(book)
        
        # 处理图片书（只有纯图片时，且使用 parent_category，因为整个文件夹是书本本身）
        if is_image_book:
            image_files = [f for f in files if f.suffix.lower() in self.IMAGE_EXTS]
            image_paths = sorted(str(f) for f in image_files)
            book = self.create_image_book(current_path, image_paths, parent_category.id if parent_category else None)
            books.append(book)
            if parent_category:
                parent_category.books.append(book)
        
        # 递归子文件夹（如果有）
        if has_subdirs:
            for subdir in subdirs:
                self.scan_dir(subdir, current_category, ignored_exts, categories, books, category_map)
    
    def print_structure(self, categories: list, books: list):
        """清晰打印扫描到的层级结构（图片文件夹和 PDF 同级）"""
        print("\n=== 扫描结果文件结构 ===\n")
        
        def print_category(cat: Category, indent: str = ""):
            print(f"{indent}分类: {cat.name}  (路径: {cat.path})")
            
            # 打印分类下的书籍
            for book in cat.books:
                if isinstance(book, Book):
                    if book.type == "image_book":
                        print(f"{indent}  ├─ 书籍: {book.title} (图片包, {book.page_count}页, 封面: {book.cover_path or '无'})")
                    else:
                        print(f"{indent}  ├─ 书籍: {book.title}.pdf (PDF, inode: {book.inode or '无'})")
            
            # 递归打印子分类
            for sub_cat in sorted(cat.sub_categories, key=lambda c: c.name):
                print_category(sub_cat, indent + "  ")
        
        # 打印根目录书籍
        top_books = [b for b in books if not hasattr(b, 'category_id') or b.category_id is None]
        if top_books:
            print("根目录直接书籍:")
            for book in sorted(top_books, key=lambda b: b.title):
                if book.type == "image_book":
                    print(f"  ├─ 书籍: {book.title} (图片包, {book.page_count}页)")
                else:
                    print(f"  ├─ 书籍: {book.title}.pdf (PDF)")
            print("")
        
        # 打印分类
        if categories:
            for cat in sorted(categories, key=lambda c: c.name):
                print_category(cat)
        else:
            if not top_books:
                print("未扫描到任何书籍或分类")
        
        print(f"\n统计: 发现 {len(categories)} 个分类, {len(books)} 本书籍")
    
    def scan(self):
        """扫描文件系统并返回根分类对象，包含所有子分类和书籍"""
        root_path, ignored_exts = self.load_settings()
        print(f"开始扫描路径: {root_path}")
        print(f"忽略文件类型: {ignored_exts or '无'}")
        
        categories = []
        books = []
        category_map = {}
        
        # 创建根分类
        root_category = self.create_category(root_path, None)
        categories.append(root_category)
        category_map[root_category.id] = root_category
        
        # 扫描根目录
        # 注意：我们直接使用root_category作为当前分类，不创建新的分类
        # 这样可以避免在根目录下重复创建分类
        try:
            entries = list(root_path.iterdir())
        except PermissionError:
            print(f"权限不足，跳过: {root_path}")
            return root_category
        
        # 排除隐藏文件/文件夹
        non_hidden = [e for e in entries if not e.name.startswith('.')]
        subdirs = [e for e in non_hidden if e.is_dir()]
        files = [e for e in non_hidden if e.is_file()]
        
        # 处理子目录
        for subdir in subdirs:
            self.scan_dir(subdir, root_category, ignored_exts, categories, books, category_map)
        
        # 处理根目录下的文件
        pdf_files = [f for f in files if f.suffix.lower() == ".pdf"]
        
        # 计算有效扩展名（排除 ignored）
        effective_exts = {
            f.suffix.lower() for f in files
            if f.suffix.lower() not in ignored_exts
        }
        
        # 是否为纯图片书（无子文件夹 + 至少有图片 + 有效文件全为图片）
        is_image_book = (
            not subdirs and
            any(f.suffix.lower() in self.IMAGE_EXTS for f in files) and
            effective_exts.issubset(self.IMAGE_EXTS)
        )
        
        # 处理 PDF 书本
        for pdf_path in pdf_files:
            book = self.create_pdf_book(pdf_path, root_category.id)
            books.append(book)
            root_category.books.append(book)
        
        # 处理图片书
        if is_image_book:
            image_files = [f for f in files if f.suffix.lower() in self.IMAGE_EXTS]
            image_paths = sorted(str(f) for f in image_files)
            book = self.create_image_book(root_path, image_paths, root_category.id)
            books.append(book)
            root_category.books.append(book)
        
        # 只返回根分类对象，包含所有子分类和书籍
        return root_category