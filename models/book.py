from typing import Optional, List, Dict, Any
from datetime import datetime
from pydantic import BaseModel, Field

# Pydantic 模型用于请求和响应验证
class BookBase(BaseModel):
    """书籍基础模型"""
    title: str
    path: str
    type: str = Field(default="image_book", description="书籍类型：image_book 或 pdf_book")
    cover_path: Optional[str] = None
    page_count: int = 0
    pages: List[str] = []
    inode: Optional[str] = None
    device_id: Optional[str] = None
    optimization_strategy: int = Field(default=0, description="优化策略：0=未知, 1=仅原图, 2=建议优化, 3=强制优化")
    page_dimension_type: Optional[str] = Field(default=None, description="页面尺寸类型：如 HD, 2K, 4K 或具体宽度")

class BookCreate(BookBase):
    """创建书籍模型"""
    id: Optional[str] = None

class BookUpdate(BaseModel):
    """更新书籍模型"""
    title: Optional[str] = None
    path: Optional[str] = None
    type: Optional[str] = None
    cover_path: Optional[str] = None
    page_count: Optional[int] = None
    pages: Optional[List[str]] = None
    inode: Optional[str] = None
    device_id: Optional[str] = None
    optimization_strategy: Optional[int] = None
    page_dimension_type: Optional[str] = None

class BookResponse(BaseModel):
    """书籍响应模型"""
    id: str
    title: str
    path: str
    type: str
    cover_path: Optional[str] = None
    page_count: int = 0
    inode: Optional[str] = None
    device_id: Optional[str] = None
    created_at: Optional[str] = None
    updated_at: Optional[str] = None
    is_deleted: bool = False
    deleted_at: Optional[str] = None
    optimization_strategy: int = Field(default=0, description="优化策略：0=未知, 1=仅原图, 2=建议优化, 3=强制优化")
    page_dimension_type: Optional[str] = Field(default=None, description="页面尺寸类型：如 HD, 2K, 4K 或具体宽度")
    is_favorite: bool = Field(default=False, description="是否收藏")

    class Config:
        """配置"""
        from_attributes = True

class Book:
    """书籍数据模型"""

    def __init__(self, id: str, title: str, path: str, book_type: str):
        self.id = id
        self.title = title
        self.path = path
        self.type = book_type  # image_book / pdf_book
        self.cover_path: Optional[str] = None
        self.page_count: int = 0
        self.pages: List[str] = []

        # === 新增字段 ===
        self.inode: Optional[str] = None              # 文件系统 inode
        self.device_id: Optional[str] = None        # 设备 ID
        self.created_at: Optional[str] = None       # 创建时间
        self.updated_at: Optional[str] = None       # 更新时间
        self.is_deleted: bool = False               # 软删除标记
        self.deleted_at: Optional[str] = None       # 删除时间
        self.optimization_strategy: int = 0         # 优化策略：0=未知, 1=仅原图, 2=建议优化, 3=强制优化
        self.page_dimension_type: Optional[str] = None  # 页面尺寸类型：如 HD, 2K, 4K 或具体宽度
        self.is_favorite: bool = False              # 是否收藏

    def to_dict(self) -> Dict[str, Any]:
        """转换为字典"""
        return {
            'id': self.id,
            'title': self.title,
            'path': self.path,
            'type': self.type,
            'cover_path': self.cover_path,
            'page_count': self.page_count,
            'pages': self.pages,
            # === 新增字段 ===
            'inode': self.inode,
            'device_id': self.device_id,
            'created_at': self.created_at,
            'updated_at': self.updated_at,
            'is_deleted': self.is_deleted,
            'deleted_at': self.deleted_at,
            'optimization_strategy': self.optimization_strategy,
            'page_dimension_type': self.page_dimension_type,
            'is_favorite': self.is_favorite
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'Book':
        """从字典创建Book对象"""
        book = cls(
            id=data.get('id', ''),
            title=data.get('title', ''),
            path=data.get('path', ''),
            book_type=data.get('type', 'image_book')
        )
        book.cover_path = data.get('cover_path')
        book.page_count = data.get('page_count', 0)
        book.pages = data.get('pages', [])
        # === 新增字段 ===
        book.inode = data.get('inode')
        book.device_id = data.get('device_id')
        book.created_at = data.get('created_at')
        book.updated_at = data.get('updated_at')
        book.is_deleted = data.get('is_deleted', False)
        book.deleted_at = data.get('deleted_at')
        book.optimization_strategy = data.get('optimization_strategy', 0)
        book.page_dimension_type = data.get('page_dimension_type')
        book.is_favorite = data.get('is_favorite', False)
        return book
