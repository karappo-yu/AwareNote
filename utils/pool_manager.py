import threading
from concurrent.futures import ThreadPoolExecutor
import time
import asyncio

# 添加项目根目录到导入路径
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))

# 导入配置模块
from config.config import get as get_config

# 全局线程池相关变量
_thread_pool = None
_pool_lock = threading.Lock()
_last_used_time = None  # 最后一次使用时间
_idle_check_task = None  # 空闲检查任务

# 正在处理的任务集合，用于避免重复任务
_processing_tasks = set()
_processing_lock = threading.Lock()

# 任务计数器，用于跟踪正在执行的任务数
_task_counter = 0
_task_counter_lock = threading.Lock()

# 空闲检查间隔（秒）
IDLE_CHECK_INTERVAL = 30

async def _check_idle_timeout():
    """检查线程池是否空闲超时，超时则关闭"""
    while True:
        await asyncio.sleep(IDLE_CHECK_INTERVAL)  # 每30秒检查一次
        
        global _thread_pool, _last_used_time
        with _pool_lock:
            if _thread_pool is not None:
                # 获取空闲超时设置
                idle_timeout = get_config('thread_pool_idle_timeout', 300)  # 默认5分钟
                
                # 检查是否超时
                if _last_used_time is not None:
                    current_time = time.time()
                    if current_time - _last_used_time > idle_timeout:
                        # 检查是否有正在执行的任务
                        with _task_counter_lock:
                            if _task_counter == 0:
                                print("Thread pool idle timeout, shutting down...")
                                _thread_pool.shutdown(wait=True)
                                _thread_pool = None
                                _last_used_time = None

async def submit_task_to_pool(func, *args, **kwargs):
    """提交任务到线程池，实现按需创建和空闲超时回收"""
    global _thread_pool, _last_used_time, _idle_check_task
    
    # 获取线程池
    pool = get_thread_pool()
    if pool is None:
        # 单线程模式，直接执行
        return func(*args, **kwargs)
    
    # 记录最后使用时间
    _last_used_time = time.time()
    
    # 启动空闲检查任务（如果尚未启动）
    if _idle_check_task is None:
        with _pool_lock:  # 复用已有的锁
            if _idle_check_task is None:
                _idle_check_task = asyncio.create_task(_check_idle_timeout())
    
    # 增加任务计数器
    with _task_counter_lock:
        global _task_counter
        _task_counter += 1
    
    try:
        # 使用异步方式等待任务完成
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(pool, func, *args, **kwargs)
    finally:
        # 减少任务计数器
        with _task_counter_lock:
            _task_counter -= 1


def get_thread_pool():
    """获取全局线程池（按需创建）"""
    global _thread_pool
    if _thread_pool is None:
        with _pool_lock:
            if _thread_pool is None:
                # 从配置文件中读取线程池配置
                use_thread_pool = get_config('use_thread_pool', True)
                max_workers = get_config('thread_pool_max_workers', 2)
                
                if use_thread_pool:
                    # 创建线程池
                    print("Creating thread pool...")
                    _thread_pool = ThreadPoolExecutor(max_workers=max_workers)
                else:
                    # 单线程模式，返回None
                    return None
    return _thread_pool


def shutdown_thread_pool():
    """关闭线程池"""
    global _thread_pool, _idle_check_task
    
    # 取消空闲检查任务
    if _idle_check_task is not None:
        _idle_check_task.cancel()
        _idle_check_task = None
    
    # 关闭线程池
    if _thread_pool is not None:
        with _pool_lock:
            if _thread_pool is not None:
                print("Shutting down thread pool...")
                _thread_pool.shutdown(wait=True)
                _thread_pool = None
                global _last_used_time
                _last_used_time = None


def get_processing_tasks():
    """获取正在处理的任务集合"""
    return _processing_tasks


def get_processing_lock():
    """获取正在处理的任务集合的锁"""
    return _processing_lock
