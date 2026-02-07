import sqlite3
import json
from typing import List, Optional
from models.book import Book
from models.category import Category

class Database:
    """SQLite 数据库类，用于持久化存储书籍和分类数据"""
    
    def __init__(self, db_path: str = "database.db"):
        self.db_path = db_path
        self._init_db()
    
    def _init_db(self) -> None:
        """初始化数据库，创建表结构"""
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            
            # 创建书籍表
            cursor.execute('''
                CREATE TABLE IF NOT EXISTS books (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    path TEXT NOT NULL,
                    type TEXT NOT NULL DEFAULT 'image_book',
                    cover_path TEXT,
                    page_count INTEGER DEFAULT 0,
                    pages TEXT DEFAULT '[]',
                    inode TEXT,
                    device_id TEXT,
                    created_at TEXT DEFAULT (datetime('now', 'localtime')),
                    updated_at TEXT DEFAULT (datetime('now', 'localtime')),
                    is_deleted INTEGER DEFAULT 0,
                    deleted_at TEXT,
                    optimization_strategy INTEGER DEFAULT 0,
                    page_dimension_type TEXT,
                    is_favorite INTEGER DEFAULT 0
                )
            ''')
            
            # 创建分类表
            cursor.execute('''
                CREATE TABLE IF NOT EXISTS categories (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    path TEXT NOT NULL,
                    created_at TEXT DEFAULT (datetime('now', 'localtime')),
                    updated_at TEXT DEFAULT (datetime('now', 'localtime')),
                    is_deleted INTEGER DEFAULT 0,
                    deleted_at TEXT
                )
            ''')
            
            # 创建实体关系表
            cursor.execute('''
                CREATE TABLE IF NOT EXISTS entity_relations (
                    id TEXT PRIMARY KEY,
                    parent_id TEXT NOT NULL,
                    child_id TEXT NOT NULL,
                    relation_type TEXT NOT NULL,
                    created_at TEXT DEFAULT (datetime('now', 'localtime')),
                    UNIQUE (parent_id, child_id)
                )
            ''')
            
            # 创建自定义分类表
            cursor.execute('''
                CREATE TABLE IF NOT EXISTS custom_categories (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT,
                    book_count INTEGER DEFAULT 0,
                    created_at TEXT DEFAULT (datetime('now', 'localtime')),
                    updated_at TEXT DEFAULT (datetime('now', 'localtime')),
                    is_deleted INTEGER DEFAULT 0,
                    deleted_at TEXT
                )
            ''')
            
            # 创建书籍和自定义分类的关系表
            cursor.execute('''
                CREATE TABLE IF NOT EXISTS book_custom_category (
                    id TEXT PRIMARY KEY,
                    book_id TEXT NOT NULL,
                    custom_category_id TEXT NOT NULL,
                    created_at TEXT DEFAULT (datetime('now', 'localtime')),
                    FOREIGN KEY (book_id) REFERENCES books (id),
                    FOREIGN KEY (custom_category_id) REFERENCES custom_categories (id),
                    UNIQUE (book_id, custom_category_id)
                )
            ''')
            
            # 设置时区为北京时间 (UTC+8)
            cursor.execute('''
                PRAGMA timezone = '+08:00';
            ''')
            
            # 创建触发器，在更新书籍时自动更新updated_at
            cursor.execute('''
                CREATE TRIGGER IF NOT EXISTS update_book_timestamp
                AFTER UPDATE ON books
                FOR EACH ROW
                BEGIN
                    UPDATE books SET updated_at = datetime('now', 'localtime') WHERE id = NEW.id;
                END
            ''')
            
            # 创建触发器，在更新分类时自动更新updated_at
            cursor.execute('''
                CREATE TRIGGER IF NOT EXISTS update_category_timestamp
                AFTER UPDATE ON categories
                FOR EACH ROW
                BEGIN
                    UPDATE categories SET updated_at = datetime('now', 'localtime') WHERE id = NEW.id;
                END
            ''')
            
            # 创建触发器，在更新自定义分类时自动更新updated_at
            cursor.execute('''
                CREATE TRIGGER IF NOT EXISTS update_custom_category_timestamp
                AFTER UPDATE ON custom_categories
                FOR EACH ROW
                BEGIN
                    UPDATE custom_categories SET updated_at = datetime('now', 'localtime') WHERE id = NEW.id;
                END
            ''')
            
            conn.commit()
    
    # 书籍相关操作
    def get_all_books(self) -> List[Book]:
        """获取所有书籍"""
        global _category_tree_cache
        if not _category_tree_cache:
            return []
        
        # 从缓存中收集所有书籍
        all_books = []
        
        def collect_books(category):
            # 收集当前分类的书籍
            for book in category.books:
                all_books.append(book)
            # 递归收集子分类的书籍
            for sub_category in category.sub_categories:
                collect_books(sub_category)
        
        # 遍历所有根分类收集书籍
        for root_category in _category_tree_cache:
            collect_books(root_category)
        
        return all_books
    
    def get_book_by_id(self, book_id: str) -> Optional[Book]:
        """根据 ID 获取书籍"""
        global _category_tree_cache
        if not _category_tree_cache:
            return None
        
        # 从缓存中查找书籍
        def find_book(category, target_id):
            # 检查当前分类的书籍
            for book in category.books:
                if book.id == target_id:
                    return book
            # 递归检查子分类
            for sub_category in category.sub_categories:
                result = find_book(sub_category, target_id)
                if result:
                    return result
            return None
        
        # 遍历所有根分类查找目标书籍
        for root_category in _category_tree_cache:
            book = find_book(root_category, book_id)
            if book:
                return book
        return None

    def get_book_by_id_from_db(self, book_id: str) -> Optional[Book]:
        """直接从数据库中根据 ID 获取书籍"""
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute('''
                SELECT id, title, path, type, cover_path, page_count, pages, inode, device_id, created_at, updated_at, optimization_strategy, page_dimension_type, is_favorite FROM books
                WHERE id = ? AND is_deleted = 0
            ''', (book_id,))
            
            row = cursor.fetchone()
            if row:
                book_data = {
                    'id': row[0],
                    'title': row[1],
                    'path': row[2],
                    'type': row[3],
                    'cover_path': row[4],
                    'page_count': row[5],
                    'pages': json.loads(row[6]) if row[6] else [],
                    'inode': row[7],
                    'device_id': row[8],
                    'created_at': row[9],
                    'updated_at': row[10],
                    'optimization_strategy': row[11] if len(row) > 11 else 0,
                    'page_dimension_type': row[12] if len(row) > 12 else None,
                    'is_favorite': row[13] == 1 if len(row) > 13 else False,
                    'is_deleted': False,
                    'deleted_at': None
                }
                return Book.from_dict(book_data)
            return None
    
    def add_book(self, book: Book) -> None:
        """添加新书籍"""
        # 检查是否已存在相同 ID 的书籍
        if self.get_book_by_id(book.id):
            raise ValueError(f"Book with ID {book.id} already exists")
        
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute('''
                INSERT INTO books (id, title, path, type, cover_path, page_count, pages, inode, device_id, is_deleted, deleted_at, optimization_strategy, page_dimension_type, is_favorite)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ''', (
                book.id,
                book.title,
                book.path,
                book.type,
                book.cover_path,
                book.page_count,
                json.dumps(book.pages),
                book.inode,
                book.device_id,
                1 if book.is_deleted else 0,
                book.deleted_at,
                book.optimization_strategy,
                book.page_dimension_type,
                1 if getattr(book, 'is_favorite', False) else 0
            ))
            conn.commit()
    
    def update_book(self, book: Book) -> int:
        """更新书籍信息
        
        Args:
            book: 书籍对象
            
        Returns:
            int: 1 if updated, 0 if no changes needed
        """
        # 直接从数据库中获取当前书籍信息
        current_book = self.get_book_by_id_from_db(book.id)
        if not current_book:
            raise ValueError(f"Book with ID {book.id} not found")
        
        # 检查是否有需要更新的字段
        need_update = False
        
        # 对于PDF书籍，检查是否有内容变化
        if book.type == "pdf_book":
            # PDF书籍内容通常不会变化，所以不需要更新
            return 0
        
        # 对于image books，检查page_count或pages是否有变化
        current_page_count = current_book.page_count
        current_pages = current_book.pages
        
        if current_page_count != book.page_count or current_pages != book.pages:
            need_update = True
        
        if need_update:
            # 执行数据库更新，保留现有的 is_favorite 值
            with sqlite3.connect(self.db_path) as conn:
                cursor = conn.cursor()
                cursor.execute('''
                    UPDATE books
                    SET title = ?, path = ?, type = ?, cover_path = ?, page_count = ?, pages = ?, inode = ?, device_id = ?, is_deleted = ?, deleted_at = ?, optimization_strategy = ?, page_dimension_type = ?
                    WHERE id = ?
                ''', (
                    book.title,
                    book.path,
                    book.type,
                    book.cover_path,
                    book.page_count,
                    json.dumps(book.pages),
                    book.inode,
                    book.device_id,
                    1 if book.is_deleted else 0,
                    book.deleted_at,
                    book.optimization_strategy,
                    book.page_dimension_type,
                    book.id
                ))
                conn.commit()
            return 1
        return 0
    
    def update_book_favorite_status(self, book_id: str, is_favorite: bool) -> bool:
        """更新书籍的收藏状态
        
        Args:
            book_id: 书籍ID
            is_favorite: 是否收藏
            
        Returns:
            bool: True if updated successfully
        """
        # 检查书籍是否存在
        current_book = self.get_book_by_id_from_db(book_id)
        if not current_book:
            raise ValueError(f"Book with ID {book_id} not found")
        
        # 执行数据库更新
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute('''
                UPDATE books
                SET is_favorite = ?
                WHERE id = ?
            ''', (
                1 if is_favorite else 0,
                book_id
            ))
            conn.commit()
        
        return True
    
    def delete_book(self, book_id: str) -> None:
        """删除书籍（真实删除）"""
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            
            # 1. 删除 entity_relations 表中与该书籍相关的所有记录
            cursor.execute('''
                DELETE FROM entity_relations
                WHERE parent_id = ? OR child_id = ?
            ''', (book_id, book_id))
            
            # 2. 查找该书籍关联的所有自定义分类
            cursor.execute('''
                SELECT custom_category_id FROM book_custom_category
                WHERE book_id = ?
            ''', (book_id,))
            related_custom_categories = cursor.fetchall()
            
            # 3. 删除 book_custom_category 表中与该书籍相关的所有记录
            cursor.execute('''
                DELETE FROM book_custom_category
                WHERE book_id = ?
            ''', (book_id,))
            
            # 4. 更新相关自定义分类的书籍数量
            from datetime import datetime
            now = datetime.now().isoformat()
            for (custom_category_id,) in related_custom_categories:
                cursor.execute('''
                    UPDATE custom_categories
                    SET book_count = MAX(0, book_count - 1), updated_at = ?
                    WHERE id = ?
                ''', (now, custom_category_id))
            
            # 5. 删除书籍本身
            cursor.execute('''
                DELETE FROM books
                WHERE id = ?
            ''', (book_id,))
            
            conn.commit()
        
        # 删除书籍相关缓存
        try:
            from utils.cache_utils import clear_book_cache
            clear_book_cache(book_id)
        except Exception as e:
            print(f"Error clearing cache for book {book_id}: {e}")
    
    # 分类相关操作
    def _build_category_tree(self, root_category_ids=None):
        """构建完整的分类树（内部方法）
        
        Args:
            root_category_ids: 根分类ID列表，如果为None则返回所有根分类
        
        Returns:
            构建好的分类树
        """
        # 首先从数据库构建完整的分类树
        all_root_categories = self._build_category_tree_from_db()
        
        # 如果指定了根分类ID，返回指定的根分类
        if root_category_ids:
            # 构建分类ID到分类对象的映射
            category_map = {}
            
            def collect_categories(category):
                category_map[category.id] = category
                for sub in category.sub_categories:
                    collect_categories(sub)
            
            for root in all_root_categories:
                collect_categories(root)
            
            # 返回指定的根分类
            result = []
            for category_id in root_category_ids:
                if category_id in category_map:
                    result.append(category_map[category_id])
            return result
        else:
            # 否则返回所有根分类
            return all_root_categories
    
    def get_all_categories(self) -> List[Category]:
        """获取所有分类"""
        global _category_tree_cache
        if _category_tree_cache:
            return _category_tree_cache
        return []
    
    def get_category_by_id(self, category_id: str) -> Optional[Category]:
        """根据 ID 获取分类"""
        global _category_tree_cache
        if not _category_tree_cache:
            return None
        
        # 从缓存中查找分类
        def find_category(category, target_id):
            if category.id == target_id:
                return category
            for sub_category in category.sub_categories:
                result = find_category(sub_category, target_id)
                if result:
                    return result
            return None
        
        # 遍历所有根分类查找目标分类
        for root_category in _category_tree_cache:
            category = find_category(root_category, category_id)
            if category:
                return category
        return None
    
    def add_entity_relation(self, parent_id: str, child_id: str, relation_type: str) -> None:
        """添加实体关系
        
        Args:
            parent_id: 父实体ID（分类ID）
            child_id: 子实体ID（分类ID或书籍ID）
            relation_type: 关系类型，如 'category_category' 或 'category_book'
        """
        import uuid
        from datetime import datetime
        
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            # 检查关系是否已存在
            cursor.execute('''
                SELECT id FROM entity_relations 
                WHERE parent_id = ? AND child_id = ?
            ''', (parent_id, child_id))
            
            if cursor.fetchone():
                return  # 关系已存在，无需重复添加
            
            # 添加新关系
            relation_id = str(uuid.uuid4())
            now = datetime.now().isoformat()
            cursor.execute('''
                INSERT INTO entity_relations (id, parent_id, child_id, relation_type, created_at)
                VALUES (?, ?, ?, ?, ?)
            ''', (
                relation_id,
                parent_id,
                child_id,
                relation_type,
                now
            ))
            conn.commit()
    
    def remove_entity_relation(self, parent_id: str, child_id: str) -> None:
        """移除实体关系"""
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute('''
                DELETE FROM entity_relations 
                WHERE parent_id = ? AND child_id = ?
            ''', (parent_id, child_id))
            conn.commit()
    
    def get_category_children(self, category_id: str) -> List[Category]:
        """获取分类的子分类"""
        global _category_tree_cache
        if not _category_tree_cache:
            return []
        
        # 从缓存中查找分类
        def find_category(category, target_id):
            if category.id == target_id:
                return category
            for sub_category in category.sub_categories:
                result = find_category(sub_category, target_id)
                if result:
                    return result
            return None
        
        # 遍历所有根分类查找目标分类
        for root_category in _category_tree_cache:
            category = find_category(root_category, category_id)
            if category:
                return category.sub_categories
        return []
    
    def clear_cache(self):
        """清空分类树缓存"""
        global _category_tree_cache
        _category_tree_cache = None
    
    def _build_category_tree_from_db(self):
        """从数据库构建分类树
        
        从数据库中读取所有数据，构建完整的分类树并返回根分类列表
        
        Returns:
            List[Category]: 根分类列表
        """
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            
            # 1. 一次性获取所有必要的数据
            
            # 获取所有未删除的分类
            cursor.execute('''
                SELECT id, name, path, created_at, updated_at FROM categories
                WHERE is_deleted = 0
            ''')
            categories_data = cursor.fetchall()
            
            # 获取所有未删除的书籍
            cursor.execute('''
                SELECT id, title, path, type, cover_path, page_count, pages, inode, device_id, created_at, updated_at, optimization_strategy, page_dimension_type, is_favorite FROM books
                WHERE is_deleted = 0
            ''')
            books_data = cursor.fetchall()
            
            # 获取所有实体关系
            cursor.execute('''
                SELECT parent_id, child_id, relation_type FROM entity_relations
            ''')
            relations_data = cursor.fetchall()
        
        # 2. 构建映射
        category_map = {}
        book_map = {}
        parent_children_map = {}
        category_books_map = {}
        
        # 构建分类映射
        for row in categories_data:
            category_data = {
                'id': row[0],
                'name': row[1],
                'path': row[2],
                'sub_categories': [],
                'books': [],
                'created_at': row[3],
                'updated_at': row[4],
                'is_deleted': False,
                'deleted_at': None
            }
            category = Category.from_dict(category_data)
            category_map[category.id] = category
            # 初始化父子关系映射
            parent_children_map[row[0]] = []
        
        # 构建书籍映射
        for row in books_data:
            # 延迟加载 pages 字段
            book_data = {
                'id': row[0],
                'title': row[1],
                'path': row[2],
                'type': row[3],
                'cover_path': row[4],
                'page_count': row[5],
                'pages': json.loads(row[6]) if row[6] else [],
                'inode': row[7],
                'device_id': row[8],
                'created_at': row[9],
                'updated_at': row[10],
                'optimization_strategy': row[11] if len(row) > 11 else 0,
                'page_dimension_type': row[12] if len(row) > 12 else None,
                'is_favorite': row[13] == 1 if len(row) > 13 else False,
                'is_deleted': False,
                'deleted_at': None
            }
            book = Book.from_dict(book_data)
            book_map[book.id] = book
        
        # 构建关系映射
        for parent_id, child_id, relation_type in relations_data:
            if relation_type == 'category_category':
                if parent_id in parent_children_map:
                    parent_children_map[parent_id].append(child_id)
            elif relation_type == 'category_book':
                if parent_id not in category_books_map:
                    category_books_map[parent_id] = []
                category_books_map[parent_id].append(child_id)
        
        # 3. 构建完整的分类树
        # 使用 BFS 构建，避免递归
        from collections import deque
        
        # 标记已处理的分类，避免重复处理
        visited = set()
        
        # 构建所有分类的子分类和书籍
        for category_id, category in category_map.items():
            if category_id in visited:
                continue
                
            queue = deque([category])
            visited.add(category_id)
            
            while queue:
                current_category = queue.popleft()
                
                # 获取子分类
                children = []
                if current_category.id in parent_children_map:
                    for child_id in parent_children_map[current_category.id]:
                        if child_id in category_map and child_id not in visited:
                            child_category = category_map[child_id]
                            children.append(child_category)
                            queue.append(child_category)
                            visited.add(child_id)
                current_category.sub_categories = children
                
                # 获取书籍
                if current_category.id in category_books_map:
                    current_category.books = [book_map[book_id] for book_id in category_books_map[current_category.id] if book_id in book_map]
                else:
                    current_category.books = []
        
        # 4. 构建根分类列表
        root_categories = []
        for category_id, category in category_map.items():
            # 检查是否为根分类（没有父分类）
            is_root = True
            for parent_id, children in parent_children_map.items():
                if category_id in children:
                    is_root = False
                    break
            if is_root:
                root_categories.append(category)
        
        return root_categories
    
    def build_cache(self, category=None):
        """构建分类树缓存
        
        Args:
            category: 可选的分类对象，如果提供，则直接使用它作为缓存，而不是从数据库构建
        
        从数据库中读取所有数据，构建完整的分类树并缓存到内存中
        或者直接使用提供的分类对象作为缓存
        """
        global _category_tree_cache
        
        # 如果提供了分类对象，直接使用它作为缓存
        if category:
            # 确保缓存是一个列表（根分类列表）
            if isinstance(category, list):
                _category_tree_cache = category
            else:
                _category_tree_cache = [category]
            return
        
        # 否则，从数据库构建缓存
        root_categories = self._build_category_tree_from_db()
        
        # 缓存构建好的分类树
        _category_tree_cache = root_categories
    
    def get_category_books(self, category_id: str) -> List[Book]:
        """获取分类下的书籍"""
        global _category_tree_cache
        if not _category_tree_cache:
            return []
        
        # 从缓存中查找分类
        def find_category(category, target_id):
            if category.id == target_id:
                return category
            for sub_category in category.sub_categories:
                result = find_category(sub_category, target_id)
                if result:
                    return result
            return None
        
        # 遍历所有根分类查找目标分类
        for root_category in _category_tree_cache:
            category = find_category(root_category, category_id)
            if category:
                    # 直接返回分类的书籍列表
                    return category.books
        return []
    
    def add_category(self, category: Category) -> None:
        """添加新分类"""
        # 检查是否已存在相同 ID 的分类
        if self.get_category_by_id(category.id):
            raise ValueError(f"Category with ID {category.id} already exists")
        
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute('''
                INSERT INTO categories (id, name, path, is_deleted, deleted_at)
                VALUES (?, ?, ?, ?, ?)
            ''', (
                category.id,
                category.name,
                category.path,
                1 if category.is_deleted else 0,
                category.deleted_at
            ))
            conn.commit()
    
    def update_category(self, category: Category) -> None:
        """更新分类信息"""
        if not self.get_category_by_id(category.id):
            raise ValueError(f"Category with ID {category.id} not found")
        
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute('''
                UPDATE categories
                SET name = ?, path = ?, is_deleted = ?, deleted_at = ?
                WHERE id = ?
            ''', (
                category.name,
                category.path,
                1 if category.is_deleted else 0,
                category.deleted_at,
                category.id
            ))
            conn.commit()
    
    def delete_category(self, category_id: str) -> None:
        """删除分类（真实删除）"""
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute('''
                DELETE FROM categories
                WHERE id = ?
            ''', (category_id,))
            conn.commit()
    
    # 自定义分类相关操作
    def get_all_custom_categories(self) -> list:
        """获取所有自定义分类"""
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute('''
                SELECT * FROM custom_categories
            ''')
            custom_categories = []
            for row in cursor.fetchall():
                custom_category = {
                    'id': row[0],
                    'name': row[1],
                    'description': row[2],
                    'book_count': row[3],
                    'created_at': row[4],
                    'updated_at': row[5],
                    'is_deleted': False,
                    'deleted_at': None
                }
                custom_categories.append(custom_category)
            return custom_categories
    
    def get_custom_category_by_id(self, custom_category_id: str) -> dict:
        """根据 ID 获取自定义分类"""
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute('''
                SELECT * FROM custom_categories WHERE id = ?
            ''', (custom_category_id,))
            row = cursor.fetchone()
            if row:
                return {
                    'id': row[0],
                    'name': row[1],
                    'description': row[2],
                    'book_count': row[3],
                    'created_at': row[4],
                    'updated_at': row[5],
                    'is_deleted': False,
                    'deleted_at': None
                }
            return None
    
    def add_custom_category(self, custom_category: dict) -> None:
        """添加自定义分类"""
        # 检查是否已存在相同 ID 的自定义分类
        if self.get_custom_category_by_id(custom_category['id']):
            raise ValueError(f"Custom category with ID {custom_category['id']} already exists")
        
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute('''
                INSERT INTO custom_categories (id, name, description, book_count, is_deleted, deleted_at)
                VALUES (?, ?, ?, ?, ?, ?)
            ''', (
                custom_category['id'],
                custom_category['name'],
                custom_category.get('description'),
                custom_category.get('book_count', 0),
                1 if custom_category.get('is_deleted', False) else 0,
                custom_category.get('deleted_at')
            ))
            conn.commit()
    
    def update_custom_category(self, custom_category: dict) -> None:
        """更新自定义分类"""
        if not self.get_custom_category_by_id(custom_category['id']):
            raise ValueError(f"Custom category with ID {custom_category['id']} not found")
        
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute('''
                UPDATE custom_categories
                SET name = ?, description = ?, book_count = ?, is_deleted = ?, deleted_at = ?
                WHERE id = ?
            ''', (
                custom_category['name'],
                custom_category.get('description'),
                custom_category.get('book_count', 0),
                1 if custom_category.get('is_deleted', False) else 0,
                custom_category.get('deleted_at'),
                custom_category['id']
            ))
            conn.commit()
    
    def delete_custom_category(self, custom_category_id: str) -> None:
        """删除自定义分类（真实删除）"""
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            
            # 1. 删除 book_custom_category 表中与该自定义分类相关的所有记录
            cursor.execute('''
                DELETE FROM book_custom_category
                WHERE custom_category_id = ?
            ''', (custom_category_id,))
            
            # 2. 删除自定义分类本身
            cursor.execute('''
                DELETE FROM custom_categories
                WHERE id = ?
            ''', (custom_category_id,))
            
            conn.commit()
    
    # 书籍和自定义分类的关系操作
    def add_book_to_custom_category(self, book_id: str, custom_category_id: str) -> None:
        """添加书籍到自定义分类"""
        # 检查书籍是否存在
        if not self.get_book_by_id(book_id):
            raise ValueError(f"Book with ID {book_id} not found")
        
        # 检查自定义分类是否存在
        if not self.get_custom_category_by_id(custom_category_id):
            raise ValueError(f"Custom category with ID {custom_category_id} not found")
        
        # 检查关系是否已存在
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute('''
                SELECT * FROM book_custom_category WHERE book_id = ? AND custom_category_id = ?
            ''', (book_id, custom_category_id))
            if cursor.fetchone():
                raise ValueError(f"Book {book_id} is already in custom category {custom_category_id}")
            
            # 添加关系
            import uuid
            from datetime import datetime
            now = datetime.now().isoformat()
            cursor.execute('''
                INSERT INTO book_custom_category (id, book_id, custom_category_id, created_at)
                VALUES (?, ?, ?, ?)
            ''', (str(uuid.uuid4()), book_id, custom_category_id, now))
            
            # 更新自定义分类的书籍数量
            cursor.execute('''
                UPDATE custom_categories
                SET book_count = book_count + 1, updated_at = ?
                WHERE id = ?
            ''', (now, custom_category_id))
            
            conn.commit()
    
    def remove_book_from_custom_category(self, book_id: str, custom_category_id: str) -> None:
        """从自定义分类中移除书籍"""
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            # 检查关系是否存在
            cursor.execute('''
                SELECT * FROM book_custom_category WHERE book_id = ? AND custom_category_id = ?
            ''', (book_id, custom_category_id))
            if not cursor.fetchone():
                raise ValueError(f"Book {book_id} is not in custom category {custom_category_id}")
            
            # 移除关系
            from datetime import datetime
            now = datetime.now().isoformat()
            cursor.execute('''
                DELETE FROM book_custom_category WHERE book_id = ? AND custom_category_id = ?
            ''', (book_id, custom_category_id))
            
            # 更新自定义分类的书籍数量
            cursor.execute('''
                UPDATE custom_categories
                SET book_count = MAX(0, book_count - 1), updated_at = ?
                WHERE id = ?
            ''', (now, custom_category_id))
            
            conn.commit()
    
    def get_books_in_custom_category(self, custom_category_id: str) -> list:
        """获取自定义分类中的书籍"""
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute('''
                SELECT b.id, b.title, b.path, b.type, b.cover_path, b.page_count, b.pages, b.inode, b.device_id, b.created_at, b.updated_at, b.optimization_strategy, b.page_dimension_type, b.is_favorite FROM books b
                JOIN book_custom_category bcc ON b.id = bcc.book_id
                WHERE bcc.custom_category_id = ?
            ''', (custom_category_id,))
            books = []
            for row in cursor.fetchall():
                book_data = {
                    'id': row[0],
                    'title': row[1],
                    'path': row[2],
                    'type': row[3],
                    'cover_path': row[4],
                    'page_count': row[5],
                    'pages': json.loads(row[6]) if row[6] else [],
                    'inode': row[7],
                    'device_id': row[8],
                    'created_at': row[9],
                    'updated_at': row[10],
                    'is_deleted': False,
                    'deleted_at': None,
                    'optimization_strategy': row[11] if len(row) > 11 else 0,
                    'page_dimension_type': row[12] if len(row) > 12 else None
                }
                books.append(Book.from_dict(book_data))
            return books
    
    def get_custom_categories_for_book(self, book_id: str) -> list:
        """获取书籍所属的自定义分类"""
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute('''
                SELECT cc.* FROM custom_categories cc
                JOIN book_custom_category bcc ON cc.id = bcc.custom_category_id
                WHERE bcc.book_id = ?
            ''', (book_id,))
            custom_categories = []
            for row in cursor.fetchall():
                custom_category = {
                    'id': row[0],
                    'name': row[1],
                    'description': row[2],
                    'book_count': row[3],
                    'created_at': row[4],
                    'updated_at': row[5],
                    'is_deleted': False,
                    'deleted_at': None
                }
                custom_categories.append(custom_category)
            return custom_categories

# 创建全局数据库实例
import os
from pathlib import Path

# 使用绝对路径确保数据库文件创建在正确的位置
db_path = Path(__file__).parent / "books.db"
db = Database(str(db_path))

# 全局分类树缓存
_category_tree_cache = None

# 依赖项函数，用于在路由中获取数据库实例
def get_db() -> Database:
    return db
