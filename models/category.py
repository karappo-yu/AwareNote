from typing import List, Dict, Any, Optional
from pydantic import BaseModel

# Pydantic 模型用于请求和响应验证
class CategoryBase(BaseModel):
    """分类基础模型"""
    name: str
    path: str

class CategoryCreate(CategoryBase):
    """创建分类模型"""
    id: Optional[str] = None

class CategoryUpdate(BaseModel):
    """更新分类模型"""
    name: Optional[str] = None
    path: Optional[str] = None

class CategoryResponse(CategoryBase):
    """分类响应模型"""
    id: str
    sub_categories: List['CategoryResponse'] = []
    books: List[Dict[str, Any]] = []
    created_at: Optional[str] = None
    updated_at: Optional[str] = None
    is_deleted: bool = False
    deleted_at: Optional[str] = None

    class Config:
        """配置"""
        from_attributes = True

# 解决循环引用
CategoryResponse.model_rebuild()

# 自定义分类的Pydantic模型
class CustomCategoryBase(BaseModel):
    """自定义分类基础模型"""
    name: str
    description: Optional[str] = None

class CustomCategoryCreate(CustomCategoryBase):
    """创建自定义分类模型"""
    id: Optional[str] = None

class CustomCategoryUpdate(BaseModel):
    """更新自定义分类模型"""
    name: Optional[str] = None
    description: Optional[str] = None

class CustomCategoryResponse(CustomCategoryBase):
    """自定义分类响应模型"""
    id: str
    book_count: int = 0
    created_at: Optional[str] = None
    updated_at: Optional[str] = None
    is_deleted: bool = False
    deleted_at: Optional[str] = None

    class Config:
        """配置"""
        from_attributes = True

class Category:
    """分类数据模型"""

    def __init__(self, id: str, name: str, path: str):
        self.id = id
        self.name = name
        self.path = path
        self.sub_categories: List[Category] = []
        self.books: List[Dict[str, Any]] = []

        # === 新增字段 ===
        self.created_at: Optional[str] = None       # 创建时间
        self.updated_at: Optional[str] = None       # 更新时间
        self.is_deleted: bool = False               # 软删除标记
        self.deleted_at: Optional[str] = None       # 删除时间

    def to_dict(self) -> Dict[str, Any]:
        """转换为字典"""
        return {
            'id': self.id,
            'name': self.name,
            'path': self.path,
            'sub_categories': [cat.to_dict() for cat in self.sub_categories],
            'books': self.books,
            # === 新增字段 ===
            'created_at': self.created_at,
            'updated_at': self.updated_at,
            'is_deleted': self.is_deleted,
            'deleted_at': self.deleted_at
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'Category':
        """从字典创建Category对象"""
        category = cls(
            id=data.get('id', ''),
            name=data.get('name', ''),
            path=data.get('path', '')
        )

        # 递归创建子分类
        for sub_cat_data in data.get('sub_categories', []):
            sub_cat = cls.from_dict(sub_cat_data)
            category.sub_categories.append(sub_cat)

        # 添加书籍
        category.books = data.get('books', [])

        # === 新增字段 ===
        category.created_at = data.get('created_at')
        category.updated_at = data.get('updated_at')
        category.is_deleted = data.get('is_deleted', False)
        category.deleted_at = data.get('deleted_at')

        return category

    def add_sub_category(self, category: 'Category'):
        """添加子分类"""
        self.sub_categories.append(category)

    def add_book(self, book: Dict[str, Any]):
        """添加书籍"""
        self.books.append(book)
