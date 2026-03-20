#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(target_os = "macos")]
mod macos_app {
    use auxm::{routes, AppState, AssetCacheService, Config, DatabaseService};
    use image::GenericImageView;
    use std::net::{SocketAddr, TcpListener as StdTcpListener};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::Arc;
    use tao::event::{Event, StartCause};
    use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
    use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent};

    #[derive(Clone, Debug)]
    enum UserEvent {
        Menu(MenuId),
        Tray,
    }

    struct RuntimePaths {
        config_path: PathBuf,
        data_dir: PathBuf,
        cache_dir: PathBuf,
    }

    pub fn run() {
        init_tracing();

        let runtime_paths = match prepare_runtime_paths() {
            Ok(paths) => paths,
            Err(err) => {
                eprintln!("failed to prepare runtime paths: {err}");
                return;
            }
        };

        let config = Config::load(&runtime_paths.config_path);
        let (listener, bind_addr, probe_addr) = match bind_listener(&config) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("failed to bind listener: {err}");
                return;
            }
        };

        start_backend(listener, config.clone());

        let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
        event_loop.set_activation_policy(ActivationPolicy::Accessory);
        let proxy = event_loop.create_proxy();
        install_event_handlers(proxy);

        let mut tray_icon = None;
        let web_url = format!("http://{}", probe_addr);
        let settings_helper = helper_app_path();
        let open_id = MenuId::new("open-web");
        let settings_id = MenuId::new("open-settings");
        let quit_id = MenuId::new("quit");

        tracing::info!(
            "awarenotes ready: config={}, data={}, cache={}, bind=http://{}, local={}",
            runtime_paths.config_path.display(),
            runtime_paths.data_dir.display(),
            runtime_paths.cache_dir.display(),
            bind_addr,
            web_url
        );

        let _ = wait_for_server(probe_addr);

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::NewEvents(StartCause::Init) => {
                    match build_tray(&open_id, &settings_id, &quit_id) {
                        Ok(icon) => tray_icon = Some(icon),
                        Err(err) => {
                            tracing::error!("failed to create tray UI: {}", err);
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                }
                Event::UserEvent(UserEvent::Menu(id)) => {
                    if id == open_id {
                        let _ = open_in_browser(&web_url);
                    } else if id == settings_id {
                        let _ = open_settings_helper(&settings_helper, &web_url);
                    } else if id == quit_id {
                        terminate_settings_helper();
                        *control_flow = ControlFlow::Exit;
                    }
                }
                Event::UserEvent(UserEvent::Tray) => {}
                Event::LoopDestroyed => {
                    terminate_settings_helper();
                    drop(tray_icon.take());
                }
                _ => {}
            }
        });
    }

    fn init_tracing() {
        let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
        let log_filter = format!("{},sqlx=off", log_level);
        let env_filter = tracing_subscriber::EnvFilter::new(log_filter);
        let fmt_layer = tracing_subscriber::fmt::layer();

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }

    fn prepare_runtime_paths() -> Result<RuntimePaths, String> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is not set".to_string())?;
        let config_dir = home
            .join("Library")
            .join("Application Support")
            .join("awarenotes");
        let data_dir = config_dir.clone();
        let cache_dir = home.join("Library").join("Caches").join("awarenotes");
        let config_path = config_dir.join("app_config.toml");
        let database_path = data_dir.join("awarenotes.db");

        std::fs::create_dir_all(&config_dir).map_err(|err| err.to_string())?;
        std::fs::create_dir_all(&data_dir).map_err(|err| err.to_string())?;
        std::fs::create_dir_all(&cache_dir).map_err(|err| err.to_string())?;

        std::env::set_var(auxm::runtime::CONFIG_PATH_ENV, &config_path);
        std::env::set_var(
            "DATABASE_URL",
            format!("sqlite:{}", database_path.display()),
        );
        std::env::set_var(auxm::runtime::CACHE_DIR_ENV, &cache_dir);
        std::env::set_var("AUXM_DISABLE_RESTART", "1");

        Ok(RuntimePaths {
            config_path,
            data_dir,
            cache_dir,
        })
    }

    fn bind_listener(config: &Config) -> Result<(StdTcpListener, SocketAddr, SocketAddr), String> {
        let bind_addr = format!("{}:{}", config.host, config.port);
        let listener = StdTcpListener::bind(&bind_addr).map_err(|err| err.to_string())?;
        listener
            .set_nonblocking(true)
            .map_err(|err| err.to_string())?;
        let addr = listener.local_addr().map_err(|err| err.to_string())?;
        let probe_ip = match addr.ip() {
            std::net::IpAddr::V4(ip) if ip.is_unspecified() => {
                std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
            }
            std::net::IpAddr::V6(ip) if ip.is_unspecified() => {
                std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)
            }
            ip => ip,
        };
        let probe_addr = SocketAddr::new(probe_ip, addr.port());
        Ok((listener, addr, probe_addr))
    }

    fn start_backend(listener: StdTcpListener, config: Config) {
        if let Err(err) = std::thread::Builder::new()
            .name("awarenotes-backend".to_string())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(err) => {
                        tracing::error!("failed to build tokio runtime: {}", err);
                        return;
                    }
                };
                runtime.block_on(async move {
                    let listener = match tokio::net::TcpListener::from_std(listener) {
                        Ok(listener) => listener,
                        Err(err) => {
                            tracing::error!("failed to create tokio listener: {}", err);
                            return;
                        }
                    };

                    let db_service = match DatabaseService::new(&config).await {
                        Ok(service) => Arc::new(service),
                        Err(err) => {
                            tracing::error!("failed to initialize database: {}", err);
                            return;
                        }
                    };

                    let asset_cache = match AssetCacheService::new(
                        config.cache.clone(),
                        config.internal.file_io_concurrency,
                    ) {
                        Ok(service) => Arc::new(service),
                        Err(err) => {
                            tracing::error!("failed to initialize asset cache: {}", err);
                            return;
                        }
                    };

                    let state = AppState {
                        db_service,
                        asset_cache,
                    };
                    let app = routes::create_router(state, &config);
                    if let Err(err) = axum::serve(listener, app).await {
                        tracing::error!("backend server exited with error: {}", err);
                    }
                });
            })
        {
            tracing::error!("failed to spawn backend thread: {}", err);
        }
    }

    fn wait_for_server(addr: SocketAddr) -> Result<(), String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| err.to_string())?;
        runtime.block_on(async move {
            for _ in 0..60 {
                if tokio::net::TcpStream::connect(addr).await.is_ok() {
                    return Ok(());
                }
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            }
            Err(format!("timeout while waiting for {}", addr))
        })
    }

    fn install_event_handlers(proxy: EventLoopProxy<UserEvent>) {
        let tray_proxy = proxy.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            let _ = event;
            let _ = tray_proxy.send_event(UserEvent::Tray);
        }));
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let _ = proxy.send_event(UserEvent::Menu(event.id));
        }));
    }

    fn build_tray(
        open_id: &MenuId,
        settings_id: &MenuId,
        quit_id: &MenuId,
    ) -> Result<tray_icon::TrayIcon, String> {
        let menu = Menu::new();
        let open_item = MenuItem::with_id(open_id.clone(), "打开 Web 界面", true, None);
        let settings_item = MenuItem::with_id(settings_id.clone(), "设置", true, None);
        let quit_item = MenuItem::with_id(quit_id.clone(), "退出", true, None);
        menu.append_items(&[
            &open_item,
            &settings_item,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ])
        .map_err(|err| format!("failed to append tray items: {err}"))?;

        TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(load_menu_icon()?)
            .with_tooltip("awarenotes")
            .with_menu_on_left_click(true)
            .build()
            .map_err(|err| format!("failed to create tray icon: {err}"))
    }

    fn load_menu_icon() -> Result<Icon, String> {
        let bytes = include_bytes!("../../icon/menu_icon@4x.png");
        let image =
            image::load_from_memory(bytes).map_err(|err| format!("invalid menu icon: {err}"))?;
        let rgba = image.to_rgba8();
        let (width, height) = image.dimensions();
        Icon::from_rgba(rgba.into_raw(), width, height)
            .map_err(|err| format!("invalid tray icon rgba: {err}"))
    }

    fn helper_app_path() -> PathBuf {
        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        let bundled = current_exe
            .parent()
            .and_then(Path::parent)
            .map(|path| path.join("Resources").join("awarenotes-settings.app"))
            .unwrap_or_else(|| PathBuf::from("awarenotes-settings.app"));
        if bundled.exists() {
            bundled
        } else {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("native-macos/dist/awarenotes-settings.app")
        }
    }

    fn open_settings_helper(path: &Path, web_url: &str) -> Result<(), String> {
        Command::new("open")
            .arg(path)
            .arg("--args")
            .arg("--api-base")
            .arg(web_url)
            .arg("--parent-pid")
            .arg(std::process::id().to_string())
            .spawn()
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    fn terminate_settings_helper() {
        let _ = Command::new("osascript")
            .arg("-e")
            .arg("tell application id \"com.mujica.awarenotes.settings\" to quit")
            .spawn();
    }

    fn open_in_browser(url: &str) -> Result<(), String> {
        Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn main() {
    macos_app::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("awarenotes menubar app is only supported on macOS");
}
