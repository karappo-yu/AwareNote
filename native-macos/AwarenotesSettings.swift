import AppKit
import SwiftUI

struct ConfigResponse: Decodable {
    let stats: ConfigStats
    let settings: ConfigSettings
}

struct ConfigStats: Decodable {
    let total_books: Int
    let cache_size_mb: Double
    let server_status: String
    let version: String
}

struct ConfigSettings: Decodable {
    let app_name: String
    let host: String
    let port: UInt16
    let log_level: String
    let database_url: String
    let root_path: String
    let scan_paths: [String]
    let image_exts: [String]
    let min_image_count: Int
    let cover_width: UInt32
    let image_page_preview_width: UInt32
    let oversized_image_avg_pixels: UInt64
    let pdf_svg_width: UInt32
    let max_render_jobs: Int
    let http_concurrency_limit: Int
    let database_max_connections: UInt32
    let database_min_connections: UInt32
    let file_io_concurrency: Int
}

struct SavePayload: Encodable {
    let app_name: String
    let host: String
    let port: UInt16
    let log_level: String
    let database_url: String
    let scan_paths: [String]
    let image_exts: [String]
    let min_image_count: Int
    let cover_width: UInt32
    let image_page_preview_width: UInt32
    let oversized_image_avg_pixels: UInt64
    let pdf_svg_width: UInt32
    let max_render_jobs: Int
    let http_concurrency_limit: Int
    let database_max_connections: UInt32
    let database_min_connections: UInt32
    let file_io_concurrency: Int
}

struct ClearCacheResponse: Decodable {
    let success: Bool
    let space_freed_mb: Double
    let target: String
}

struct ScanEventEnvelope: Decodable {
    let kind: String
    let message: String
}

struct ScanPathRow: Identifiable {
    let id: String
    let path: String

    init(path: String) {
        self.id = path
        self.path = path
    }
}


enum SettingsTab: String, CaseIterable, Identifiable {
    case general = "常规"
    case library = "目录与扫描"
    case cache = "缓存与渲染"
    case advanced = "高级"

    var id: String { rawValue }

    var symbolName: String {
        switch self {
        case .general:
            return "gearshape"
        case .library:
            return "folder"
        case .cache:
            return "square.stack.3d.up"
        case .advanced:
            return "slider.horizontal.3"
        }
    }
}

@MainActor
final class SettingsViewModel: ObservableObject {
    @Published var selectedTab: SettingsTab = .general

    @Published var appName = ""
    @Published var host = "0.0.0.0"
    @Published var port = "3001" { didSet { scheduleAutoSave() } }
    @Published var logLevel = "info" { didSet { scheduleAutoSave() } }
    @Published var databaseURL = ""

    @Published var scanPaths: [String] = [] { didSet { scheduleAutoSave() } }
    @Published var selectedScanPath: String?
    @Published var imageExts = "jpg, jpeg, png, webp, gif" { didSet { scheduleAutoSave() } }
    @Published var minImageCount = "3" { didSet { scheduleAutoSave() } }

    @Published var coverWidth = "480" { didSet { scheduleAutoSave() } }
    @Published var imagePreviewWidth = "1600" { didSet { scheduleAutoSave() } }
    @Published var oversizedPixels = "10000000" { didSet { scheduleAutoSave() } }
    @Published var pdfSvgWidth = "1400" { didSet { scheduleAutoSave() } }
    @Published var maxRenderJobs = "4" { didSet { scheduleAutoSave() } }

    @Published var httpConcurrencyLimit = "128" { didSet { scheduleAutoSave() } }
    @Published var databaseMaxConnections = "16" { didSet { scheduleAutoSave() } }
    @Published var databaseMinConnections = "1" { didSet { scheduleAutoSave() } }
    @Published var fileIOConcurrency = "32" { didSet { scheduleAutoSave() } }

    @Published var totalBooks = "0"
    @Published var cacheSize = "0 MB"
    @Published var serverStatus = "-"
    @Published var version = "-"
    @Published var statusMessage = ""
    @Published var logs: [String] = []

    @Published var isSaving = false
    @Published var isScanning = false
    @Published var isClearingCache = false

    let apiBase: URL
    private var suppressAutoSave = false
    private var autoSaveTask: Task<Void, Never>?

    init(apiBase: URL) {
        self.apiBase = apiBase
    }

    func load() async {
        do {
            let response: ConfigResponse = try await request("/api/config", method: "GET")
            apply(response)
            statusMessage = "已连接到本地服务"
        } catch {
            statusMessage = "读取配置失败: \(error.localizedDescription)"
        }
    }

    func save() async -> Bool {
        isSaving = true
        defer { isSaving = false }

        do {
            let payload = try currentPayload()
            let response: ConfigResponse = try await request("/api/config", method: "PUT", body: payload)
            apply(response)
            statusMessage = "设置已保存。监听地址、数据库与并发参数会在下次启动后生效。"
            return true
        } catch {
            statusMessage = "保存失败: \(error.localizedDescription)"
            return false
        }
    }

    func scheduleAutoSave() {
        guard !suppressAutoSave else { return }
        autoSaveTask?.cancel()
        autoSaveTask = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(450))
            guard let self, !Task.isCancelled, !self.isScanning, !self.isClearingCache else { return }
            _ = await self.save()
        }
    }

    func scan() async {
        guard !isScanning else { return }
        let saved = await save()
        guard saved else { return }

        isScanning = true
        logs.removeAll()
        statusMessage = "开始扫描..."

        do {
            var request = URLRequest(url: endpoint("/scan/stream"))
            request.httpMethod = "GET"
            let (bytes, _) = try await URLSession.shared.bytes(for: request)
            var currentEvent = "message"
            for try await line in bytes.lines {
                if Task.isCancelled { break }
                if line.hasPrefix("event:") {
                    currentEvent = String(line.dropFirst(6)).trimmingCharacters(in: .whitespaces)
                } else if line.hasPrefix("data:") {
                    let data = String(line.dropFirst(5)).trimmingCharacters(in: .whitespaces)
                    if let json = data.data(using: .utf8),
                       let payload = try? JSONDecoder().decode(ScanEventEnvelope.self, from: json) {
                        logs.append(payload.message)
                        statusMessage = payload.message
                        if currentEvent == "complete" || currentEvent == "failed" {
                            break
                        }
                    }
                }
            }
            await load()
        } catch {
            statusMessage = "扫描失败: \(error.localizedDescription)"
        }

        isScanning = false
    }

    func clearCache(target: String = "all", label: String = "全部") async {
        guard !isClearingCache else { return }
        isClearingCache = true
        defer { isClearingCache = false }

        do {
            let path = target == "all" ? "/api/cache/clear" : "/api/cache/clear/\(target)"
            let response: ClearCacheResponse = try await request(path, method: "DELETE")
            statusMessage = String(format: "%@缓存已清理，释放 %.1f MB", label, response.space_freed_mb)
            await load()
        } catch {
            statusMessage = "清理缓存失败: \(error.localizedDescription)"
        }
    }

    func addPath() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.canCreateDirectories = true
        if panel.runModal() == .OK, let url = panel.url {
            let path = url.path
            if !scanPaths.contains(path) {
                scanPaths.append(path)
            }
        }
    }

    func removePath(_ path: String) {
        scanPaths.removeAll { $0 == path }
        if selectedScanPath == path {
            selectedScanPath = nil
        }
    }

    func openWeb() {
        NSWorkspace.shared.open(apiBase)
    }

    func openCacheDirectory() {
        NSWorkspace.shared.open(cacheDirectoryURL)
    }

    func openConfigFile() {
        NSWorkspace.shared.activateFileViewerSelecting([configFileURL])
    }

    var localWebAddress: String {
        apiBase.absoluteString
    }

    var configFileURL: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Application Support/awarenotes/app_config.toml", isDirectory: false)
    }

    var cacheDirectoryURL: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Caches/awarenotes", isDirectory: true)
    }

    private func apply(_ response: ConfigResponse) {
        suppressAutoSave = true
        defer { suppressAutoSave = false }

        totalBooks = "\(response.stats.total_books)"
        cacheSize = String(format: "%.1f MB", response.stats.cache_size_mb)
        serverStatus = response.stats.server_status
        version = response.stats.version

        appName = response.settings.app_name
        host = response.settings.host
        port = "\(response.settings.port)"
        logLevel = response.settings.log_level
        databaseURL = response.settings.database_url

        scanPaths = response.settings.scan_paths
        imageExts = response.settings.image_exts.joined(separator: ", ")
        minImageCount = "\(response.settings.min_image_count)"

        coverWidth = "\(response.settings.cover_width)"
        imagePreviewWidth = "\(response.settings.image_page_preview_width)"
        oversizedPixels = "\(response.settings.oversized_image_avg_pixels)"
        pdfSvgWidth = "\(response.settings.pdf_svg_width)"
        maxRenderJobs = "\(response.settings.max_render_jobs)"

        httpConcurrencyLimit = "\(response.settings.http_concurrency_limit)"
        databaseMaxConnections = "\(response.settings.database_max_connections)"
        databaseMinConnections = "\(response.settings.database_min_connections)"
        fileIOConcurrency = "\(response.settings.file_io_concurrency)"
    }

    private func currentPayload() throws -> SavePayload {
        let trimmedAppName = appName.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedHost = host.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedLogLevel = logLevel.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedDatabaseURL = databaseURL.trimmingCharacters(in: .whitespacesAndNewlines)

        guard !trimmedAppName.isEmpty,
              !trimmedHost.isEmpty,
              !trimmedLogLevel.isEmpty,
              !trimmedDatabaseURL.isEmpty,
              let port = UInt16(port),
              let minImageCount = Int(minImageCount),
              let coverWidth = UInt32(coverWidth),
              let imagePreviewWidth = UInt32(imagePreviewWidth),
              let oversizedPixels = UInt64(oversizedPixels),
              let pdfSvgWidth = UInt32(pdfSvgWidth),
              let maxRenderJobs = Int(maxRenderJobs),
              let httpConcurrencyLimit = Int(httpConcurrencyLimit),
              let databaseMaxConnections = UInt32(databaseMaxConnections),
              let databaseMinConnections = UInt32(databaseMinConnections),
              let fileIOConcurrency = Int(fileIOConcurrency)
        else {
            throw NSError(domain: "awarenotes.settings", code: 1, userInfo: [
                NSLocalizedDescriptionKey: "请检查文本和数值字段"
            ])
        }

        let normalizedMaxConnections = max(1, databaseMaxConnections)
        let normalizedMinConnections = min(max(1, databaseMinConnections), normalizedMaxConnections)

        return SavePayload(
            app_name: trimmedAppName,
            host: trimmedHost,
            port: max(1, port),
            log_level: trimmedLogLevel,
            database_url: trimmedDatabaseURL,
            scan_paths: scanPaths,
            image_exts: imageExts
                .split(separator: ",")
                .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                .filter { !$0.isEmpty },
            min_image_count: max(1, minImageCount),
            cover_width: max(64, coverWidth),
            image_page_preview_width: max(256, imagePreviewWidth),
            oversized_image_avg_pixels: max(1_000_000, oversizedPixels),
            pdf_svg_width: max(256, pdfSvgWidth),
            max_render_jobs: max(1, maxRenderJobs),
            http_concurrency_limit: max(1, httpConcurrencyLimit),
            database_max_connections: normalizedMaxConnections,
            database_min_connections: normalizedMinConnections,
            file_io_concurrency: max(1, fileIOConcurrency)
        )
    }

    private func endpoint(_ path: String) -> URL {
        apiBase.appendingPathComponent(path.trimmingCharacters(in: CharacterSet(charactersIn: "/")))
    }

    private func request<T: Decodable>(_ path: String, method: String, body: Encodable? = nil) async throws -> T {
        var request = URLRequest(url: endpoint(path))
        request.httpMethod = method
        if let body {
            request.httpBody = try JSONEncoder().encode(AnyEncodable(body))
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        }
        let (data, response) = try await URLSession.shared.data(for: request)
        guard let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw NSError(domain: "awarenotes.settings", code: 2, userInfo: [
                NSLocalizedDescriptionKey: "请求失败"
            ])
        }
        return try JSONDecoder().decode(T.self, from: data)
    }
}

struct AnyEncodable: Encodable {
    private let encodeImpl: (Encoder) throws -> Void

    init(_ wrapped: Encodable) {
        self.encodeImpl = wrapped.encode(to:)
    }

    func encode(to encoder: Encoder) throws {
        try encodeImpl(encoder)
    }
}

private struct SettingsContentHeightKey: PreferenceKey {
    static var defaultValue: CGFloat = 0

    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = max(value, nextValue())
    }
}

private struct WindowAccessor: NSViewRepresentable {
    @Binding var window: NSWindow?

    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        DispatchQueue.main.async {
            self.window = view.window
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        DispatchQueue.main.async {
            self.window = nsView.window
        }
    }
}

private struct SettingsSection<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(title)
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(.secondary)
            VStack(alignment: .leading, spacing: 10) {
                content
            }
            Divider()
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct SettingsRootView: View {
    @StateObject var model: SettingsViewModel
    @Environment(\.openWindow) private var openWindow
    @State private var window: NSWindow?
    @State private var currentContentHeight: CGFloat = 420

    var body: some View {
        VStack(spacing: 0) {
            tabBar
            currentTabContent
        }
        .task { await model.load() }
        .onPreferenceChange(SettingsContentHeightKey.self) { height in
            guard height > 0 else { return }
            currentContentHeight = height
            resizeWindowIfNeeded()
        }
        .onChange(of: model.selectedTab) { _, _ in resizeWindowIfNeeded() }
        .frame(width: 550)
        .background(WindowAccessor(window: $window))
        .background(Color(nsColor: .windowBackgroundColor))
    }

    var tabBar: some View {
        ZStack(alignment: .topTrailing) {
            HStack(spacing: 8) {
                ForEach(SettingsTab.allCases) { tab in
                    tabButton(for: tab)
                }
            }
            .frame(width: 508, alignment: .center)
            .frame(maxWidth: .infinity, alignment: .center)

            if model.isSaving {
                ProgressView()
                    .controlSize(.small)
                    .padding(.top, 8)
                    .padding(.trailing, 6)
            }
        }
        .padding(.horizontal, 12)
        .padding(.top, 18)
        .padding(.bottom, 16)
        .background(
            Rectangle()
                .fill(Color(nsColor: .windowBackgroundColor))
                .overlay(alignment: .bottom) {
                    Divider()
                }
        )
    }

    func tabButton(for tab: SettingsTab) -> some View {
        let isSelected = model.selectedTab == tab
        return Button {
            model.selectedTab = tab
        } label: {
            ZStack {
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .fill(isSelected ? Color.accentColor.opacity(0.14) : Color.clear)

                VStack(spacing: 6) {
                    Image(systemName: tab.symbolName)
                        .font(.system(size: 23, weight: .regular))
                    Text(tab.rawValue)
                        .font(.system(size: 12.5, weight: .medium))
                }
                .foregroundColor(isSelected ? Color(nsColor: .controlAccentColor) : .secondary)
            }
            .frame(maxWidth: .infinity, minHeight: 72, maxHeight: 72)
            .contentShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        }
        .buttonStyle(.plain)
        .frame(maxWidth: .infinity, minHeight: 72, maxHeight: 72)
    }

    @ViewBuilder
    var currentTabContent: some View {
        switch model.selectedTab {
        case .general:
            settingsForm(for: .general) {
                statusSection
                generalSections
            }
        case .library:
            settingsForm(for: .library) {
                statusSection
                librarySections
            }
        case .cache:
            settingsForm(for: .cache) {
                statusSection
                cacheSections
            }
        case .advanced:
            settingsForm(for: .advanced) {
                statusSection
                advancedSections
            }
        }
    }

    var statusSection: some View {
        SettingsSection(title: "运行状态") {
            metricRow("书籍总数", value: model.totalBooks)
            metricRow("缓存体积", value: model.cacheSize)
            LabeledContent("Web") {
                Button("打开") { model.openWeb() }
                    .buttonStyle(.bordered)
            }
            if !model.statusMessage.isEmpty {
                Text(model.statusMessage)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
        }
    }

    @ViewBuilder
    var generalSections: some View {
        SettingsSection(title: "服务") {
            settingsNumberField("端口", text: $model.port, width: 70)
            LabeledContent("日志级别") {
                Picker("", selection: $model.logLevel) {
                    Text("Trace").tag("trace")
                    Text("Debug").tag("debug")
                    Text("Info").tag("info")
                    Text("Warn").tag("warn")
                    Text("Error").tag("error")
                }
                .labelsHidden()
                .frame(width: 110)
            }
        }
    }

    @ViewBuilder
    var librarySections: some View {
        SettingsSection(title: "扫描目录") {
            if model.scanPaths.isEmpty {
                Text("还没有添加扫描目录")
                    .foregroundStyle(.secondary)
            } else {
                Table(scanPathRows, selection: $model.selectedScanPath) {
                    TableColumn("") { row in
                        Image(systemName: "folder")
                            .foregroundStyle(.secondary)
                    }
                    .width(22)

                    TableColumn("") { row in
                        Text(row.path)
                            .textSelection(.enabled)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                }
                .frame(height: 150)
                .tableStyle(.bordered)
                .alternatingRowBackgrounds(.enabled)
            }

            HStack(spacing: 0) {
                Button {
                    model.addPath()
                } label: {
                    Image(systemName: "plus")
                        .frame(width: 28, height: 24)
                }
                .buttonStyle(.borderless)

                Divider()
                    .frame(height: 18)
                    .padding(.horizontal, 4)

                Button {
                    if let path = model.selectedScanPath {
                        model.removePath(path)
                    }
                } label: {
                    Image(systemName: "minus")
                        .frame(width: 28, height: 24)
                }
                .buttonStyle(.borderless)
                .disabled(model.selectedScanPath == nil)

                Spacer()
            }
        }

        SettingsSection(title: "扫描规则") {
            settingsTextField("图片扩展名", text: $model.imageExts, width: 160)
            settingsStepperField("判定为书籍的最少图片数", text: $model.minImageCount, width: 70, range: 1...999, step: 1)
        }

        SettingsSection(title: "扫描") {
            HStack {
                Spacer()
                Button {
                    openWindow(id: "scan-logs")
                    Task { await model.scan() }
                } label: {
                    if model.isScanning {
                        ProgressView()
                            .controlSize(.small)
                    } else {
                        Text("立即扫描")
                    }
                }
                .disabled(model.isScanning)
            }
            Text("点击“立即扫描”后会弹出独立日志窗口。")
                .foregroundStyle(.secondary)
        }
    }

    @ViewBuilder
    var cacheSections: some View {
        SettingsSection(title: "缓存管理") {
            LabeledContent("详细清理选项") {
                Button("打开") {
                    openWindow(id: "cache-manager")
                }
                .buttonStyle(.bordered)
            }
        }

        SettingsSection(title: "本地文件") {
            LabeledContent("缓存目录") {
                Button("打开") { model.openCacheDirectory() }
                    .buttonStyle(.bordered)
            }
            LabeledContent("配置文件") {
                Button("打开") { model.openConfigFile() }
                    .buttonStyle(.bordered)
            }
        }

        SettingsSection(title: "图像缓存") {
            settingsStepperField("封面宽度", text: $model.coverWidth, width: 70, range: 64...4096, step: 32)
            settingsStepperField("图片页预览宽度", text: $model.imagePreviewWidth, width: 70, range: 256...8192, step: 64)
            settingsStepperField("超大图抽样平均像素阈值", text: $model.oversizedPixels, width: 90, range: 1_000_000...100_000_000, step: 500_000)
            settingsStepperField("最大渲染任务数", text: $model.maxRenderJobs, width: 70, range: 1...64, step: 1)
        }

        SettingsSection(title: "PDF") {
            settingsStepperField("SVG 渲染宽度", text: $model.pdfSvgWidth, width: 70, range: 256...4096, step: 64)
        }
    }

    @ViewBuilder
    var advancedSections: some View {
        SettingsSection(title: "HTTP") {
            settingsStepperField("HTTP 并发限制", text: $model.httpConcurrencyLimit, width: 70, range: 1...4096, step: 1)
        }

        SettingsSection(title: "数据库连接池") {
            settingsStepperField("最大连接数", text: $model.databaseMaxConnections, width: 70, range: 1...256, step: 1)
            settingsStepperField("最小连接数", text: $model.databaseMinConnections, width: 70, range: 1...256, step: 1)
        }

        SettingsSection(title: "文件与渲染") {
            settingsStepperField("文件 IO 并发", text: $model.fileIOConcurrency, width: 70, range: 1...256, step: 1)
        }
    }

    func settingsForm<Content: View>(for tab: SettingsTab, @ViewBuilder content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 18) {
            content()
            Text("服务监听、数据库路径和内部并发参数保存后不会立即重建当前后端进程，下次启动 awarenotes 生效。")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 28)
        .padding(.vertical, 22)
        .frame(maxWidth: .infinity, alignment: .leading)
        .fixedSize(horizontal: false, vertical: true)
        .background(Color(nsColor: .controlBackgroundColor))
        .background(
            GeometryReader { proxy in
                Color.clear
                    .preference(key: SettingsContentHeightKey.self, value: proxy.size.height)
            }
        )
        .id(tab)
    }

    var scanPathRows: [ScanPathRow] {
        model.scanPaths.map(ScanPathRow.init(path:))
    }

    func metricRow(_ title: String, value: String) -> some View {
        LabeledContent(title) {
            Text(value)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    func settingsTextField(_ title: String, text: Binding<String>, width: CGFloat) -> some View {
        LabeledContent(title) {
            TextField("", text: text)
                .textFieldStyle(.roundedBorder)
                .multilineTextAlignment(.trailing)
                .frame(width: width)
        }
    }

    func settingsNumberField(_ title: String, text: Binding<String>, width: CGFloat) -> some View {
        LabeledContent(title) {
            TextField("", text: text)
                .textFieldStyle(.roundedBorder)
                .multilineTextAlignment(.trailing)
                .frame(width: width)
        }
    }

    func settingsStepperField(
        _ title: String,
        text: Binding<String>,
        width: CGFloat,
        range: ClosedRange<Int>,
        step: Int
    ) -> some View {
        let value = Binding<Int>(
            get: { Int(text.wrappedValue) ?? range.lowerBound },
            set: { text.wrappedValue = "\($0)" }
        )
        return LabeledContent(title) {
            HStack(spacing: 8) {
                TextField("", value: value, formatter: integerFormatter)
                    .textFieldStyle(.roundedBorder)
                    .multilineTextAlignment(.trailing)
                    .frame(width: width)
                Stepper("", value: value, in: range, step: step)
                    .labelsHidden()
            }
        }
    }

    var integerFormatter: NumberFormatter {
        let formatter = NumberFormatter()
        formatter.numberStyle = .none
        formatter.generatesDecimalNumbers = false
        return formatter
    }

    func resizeWindowIfNeeded() {
        guard let window else { return }
        let targetSize = NSSize(width: 550, height: max(360, currentContentHeight + 78))
        guard abs(window.frame.size.height - targetSize.height) > 1 || abs(window.frame.size.width - targetSize.width) > 1 else {
            return
        }
        window.setContentSize(targetSize)
    }
}

struct ScanLogsWindowView: View {
    @ObservedObject var model: SettingsViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text(model.isScanning ? "正在扫描" : "扫描日志")
                    .font(.headline)
                Spacer()
                if model.isScanning {
                    ProgressView()
                        .controlSize(.small)
                }
            }

            ScrollView {
                LazyVStack(alignment: .leading, spacing: 6) {
                    if model.logs.isEmpty {
                        Text("扫描开始后，日志会显示在这里。")
                            .foregroundStyle(.secondary)
                    } else {
                        ForEach(model.logs.indices, id: \.self) { index in
                            Text(model.logs[index])
                                .font(.system(.body, design: .monospaced))
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }

            if !model.statusMessage.isEmpty {
                Text(model.statusMessage)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(16)
        .frame(minWidth: 620, minHeight: 360)
    }
}

struct CacheManagerWindowView: View {
    @ObservedObject var model: SettingsViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            LazyVGrid(columns: [
                GridItem(.flexible(), spacing: 12),
                GridItem(.flexible(), spacing: 12)
            ], spacing: 12) {
                cacheActionTile(target: "all", label: "全部")
                cacheActionTile(target: "svg", label: "SVG")
                cacheActionTile(target: "covers", label: "封面")
                cacheActionTile(target: "thumbnails", label: "缩略图")
            }
            Divider()
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
        .frame(width: 420)
    }

    func cacheActionTile(target: String, label: String) -> some View {
        HStack(alignment: .center, spacing: 10) {
            Text(cacheTileTitle(label))
                .font(.system(size: 14, weight: .semibold))
                .lineLimit(1)
            Spacer(minLength: 8)
            Button(role: target == "all" ? .destructive : nil) {
                Task { await model.clearCache(target: target, label: label) }
            } label: {
                if model.isClearingCache {
                    ProgressView()
                        .controlSize(.small)
                } else {
                    Text("执行")
                }
            }
            .buttonStyle(.bordered)
            .controlSize(.small)
            .disabled(model.isClearingCache)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity, minHeight: 56, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .fill(Color(nsColor: .controlBackgroundColor))
        )
        .overlay(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .stroke(Color(nsColor: .separatorColor).opacity(0.35), lineWidth: 1)
        )
    }

    func cacheTileTitle(_ label: String) -> String {
        switch label {
        case "全部":
            return "全部缓存"
        case "SVG":
            return "SVG"
        case "封面":
            return "封面"
        case "缩略图":
            return "缩略图"
        default:
            return label
        }
    }
}

final class SettingsAppDelegate: NSObject, NSApplicationDelegate {
    private var parentMonitor: Timer?

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)
        startParentMonitor()
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(handleWindowWillClose(_:)),
            name: NSWindow.willCloseNotification,
            object: nil
        )
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }

    func applicationWillTerminate(_ notification: Notification) {
        parentMonitor?.invalidate()
        parentMonitor = nil
        NotificationCenter.default.removeObserver(self)
    }

    @objc
    private func handleWindowWillClose(_ notification: Notification) {
        guard let window = notification.object as? NSWindow else { return }
        if window.identifier?.rawValue == "settings" {
            for otherWindow in NSApp.windows where otherWindow != window {
                otherWindow.close()
            }
        }
    }

    private func startParentMonitor() {
        guard let parentPID = parentPIDFromArguments() else { return }
        parentMonitor?.invalidate()
        parentMonitor = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { _ in
            if kill(parentPID, 0) != 0 {
                NSApp.terminate(nil)
            }
        }
        if let parentMonitor {
            RunLoop.main.add(parentMonitor, forMode: .common)
        }
    }

    private func parentPIDFromArguments() -> Int32? {
        let args = CommandLine.arguments
        guard let index = args.firstIndex(of: "--parent-pid"), index + 1 < args.count else {
            return nil
        }
        return Int32(args[index + 1])
    }
}

@main
struct AwarenotesSettingsApp: App {
    @NSApplicationDelegateAdaptor(SettingsAppDelegate.self) private var appDelegate
    @StateObject private var model = SettingsViewModel(apiBase: apiBaseURL())

    var body: some Scene {
        Window("设置", id: "settings") {
            SettingsRootView(model: model)
        }
        .windowStyle(.hiddenTitleBar)
        .defaultWindowPlacement { content, context in
            let size = content.sizeThatFits(.unspecified)
            let visible = context.defaultDisplay.visibleRect
            let origin = CGPoint(
                x: visible.midX - size.width / 2,
                y: visible.midY - size.height / 2
            )
            return WindowPlacement(origin, size: size)
        }
        .windowResizability(.contentSize)
        .defaultSize(width: 550, height: 420)

        Window("扫描日志", id: "scan-logs") {
            ScanLogsWindowView(model: model)
        }
        .windowStyle(.hiddenTitleBar)
        .windowResizability(.contentSize)
        .defaultSize(width: 680, height: 420)

        Window("缓存管理", id: "cache-manager") {
            CacheManagerWindowView(model: model)
        }
        .windowStyle(.hiddenTitleBar)
        .windowResizability(.contentSize)
        .defaultSize(width: 420, height: 170)
    }
}

func apiBaseURL() -> URL {
    let args = CommandLine.arguments
    if let index = args.firstIndex(of: "--api-base"), index + 1 < args.count,
       let url = URL(string: args[index + 1]) {
        return url
    }
    return URL(string: "http://127.0.0.1:3001")!
}
