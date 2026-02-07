#!/usr/bin/env python3
"""
AwareNote 应用入口点
创建 macOS 菜单栏应用，运行 FastAPI 后端服务
"""

import sys
import os
import threading
import time
from datetime import datetime

# 处理 PyInstaller 打包后的路径
if getattr(sys, 'frozen', False):
    # 打包后的环境
    bundle_dir = sys._MEIPASS
    # 在 macOS 应用包中，资源文件位于 Contents/Resources 目录
    if sys.platform == 'darwin':
        executable_dir = os.path.dirname(sys.executable)
        bundle_dir = os.path.join(executable_dir, '..', 'Resources')
else:
    # 开发环境
    bundle_dir = os.path.dirname(os.path.abspath(__file__))

# 添加项目根目录到导入路径
sys.path.insert(0, bundle_dir)

# 导入必要的模块
try:
    from AppKit import NSApplication, NSStatusBar, NSMenu, NSMenuItem, NSImage
    from Foundation import NSObject, NSTimer
    import objc
    
    # 导入应用模块
    from app import app
    import uvicorn
    
    # 导入配置模块
    from config.config import get as get_config
    
    # 导入数据库模块
    from database.db import get_db
    
except Exception as e:
    print(f"导入模块失败: {e}")
    sys.exit(1)

class AwareNoteAppDelegate(NSObject):
    """应用委托类"""
    
    def init(self):
        self = objc.super(AwareNoteAppDelegate, self).init()
        if self is None:
            return None
        
        # 初始化状态
        self.status_item = None
        self.server_thread = None
        self.server_running = False
        self.server_port = 8000
        
        return self
    
    def applicationDidFinishLaunching_(self, notification):
        """应用启动完成"""
        print(f"[{datetime.now()}] AwareNote 应用启动")
        
        # 创建状态栏项目
        self.create_status_item()
        
        # 启动后端服务器
        self.start_server()
    
    def create_status_item(self):
        """创建状态栏项目"""
        status_bar = NSStatusBar.systemStatusBar()
        
        # 创建状态栏项目
        self.status_item = status_bar.statusItemWithLength_(-1)
        
        # 设置图标
        icon_path = os.path.join(bundle_dir, 'icon', 'MenuIcon.icns')
        if os.path.exists(icon_path):
            icon = NSImage.alloc().initWithContentsOfFile_(icon_path)
            if icon:
                icon.setSize_((20.0, 20.0))
                self.status_item.setImage_(icon)
        
        # 不显示标题，只显示图标
        self.status_item.setTitle_("")
        
        # 创建菜单
        menu = NSMenu.alloc().init()
        
        # 添加打开应用菜单项
        open_item = NSMenuItem.alloc().initWithTitle_action_keyEquivalent_("打开应用", "openApp:", "")
        menu.addItem_(open_item)
        
        # 添加分隔线
        menu.addItem_(NSMenuItem.separatorItem())
        
        # 添加退出菜单项
        quit_item = NSMenuItem.alloc().initWithTitle_action_keyEquivalent_("退出", "quitApp:", "q")
        menu.addItem_(quit_item)
        
        # 设置菜单
        self.status_item.setMenu_(menu)
        
        # 设置目标
        open_item.setTarget_(self)
        quit_item.setTarget_(self)
    
    def openApp_(self, sender):
        """打开应用"""
        print(f"[{datetime.now()}] 打开 AwareNote 应用")
        
        # 在浏览器中打开应用
        import webbrowser
        webbrowser.open(f"http://localhost:{self.server_port}")
    
    def quitApp_(self, sender):
        """退出应用"""
        print(f"[{datetime.now()}] 退出 AwareNote 应用")
        
        # 停止服务器
        self.stop_server()
        
        # 退出应用
        NSApplication.sharedApplication().terminate_(self)
    
    def start_server(self):
        """启动后端服务器"""
        def run_server():
            print(f"[{datetime.now()}] 启动 FastAPI 服务器在端口 {self.server_port}")
            
            try:
                # 启动 uvicorn 服务器
                uvicorn.run(
                    "app:app",
                    host="0.0.0.0",
                    port=self.server_port,
                    reload=False,
                    workers=1
                )
            except Exception as e:
                print(f"服务器启动失败: {e}")
        
        # 创建并启动服务器线程
        self.server_thread = threading.Thread(target=run_server, daemon=True)
        self.server_thread.start()
        self.server_running = True
        
        print(f"[{datetime.now()}] 服务器线程已启动")
    
    def stop_server(self):
        """停止后端服务器"""
        if self.server_running and self.server_thread:
            print(f"[{datetime.now()}] 停止服务器...")
            # 由于 uvicorn 服务器在单独的线程中运行，
            # 我们可以通过设置 daemon=True 让它随主线程退出
            self.server_running = False

def main():
    """主函数"""
    print(f"[{datetime.now()}] AwareNote 入口点启动")
    
    # 创建应用实例
    app = NSApplication.sharedApplication()
    
    # 创建应用委托
    delegate = AwareNoteAppDelegate.alloc().init()
    
    # 设置应用委托
    app.setDelegate_(delegate)
    
    # 运行应用
    app.run()

if __name__ == '__main__':
    main()
