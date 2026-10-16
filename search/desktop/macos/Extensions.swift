import SwiftUI
import WebKit
import CryptoKit

struct QuillExtension: Codable, Identifiable {
    let id: String
    let name: String
    let version: String
    let description: String
    let rules: String?
    let css: String?

    func validate() throws {
        guard id.range(of: "^[a-z0-9][a-z0-9-]{0,63}$", options: .regularExpression) != nil,
              !name.isEmpty, name.count <= 80, version.count <= 40, description.count <= 1000,
              rules != nil || css != nil, (css?.utf8.count ?? 0) <= 100_000 else {
            throw EngineFailure(message: "Invalid Quill extension package.")
        }
        if let rules {
            guard let list = try JSONSerialization.jsonObject(with: Data(rules.utf8)) as? [[String: Any]],
                  !list.isEmpty, list.count <= 10_000,
                  list.allSatisfy({ ($0["action"] as? [String: Any])?["type"] as? String == "block" }) else {
                throw EngineFailure(message: "Content rules must contain only block actions.")
            }
        }
        if let css, ["url", "@", "image-set", "/", "\\"].contains(where: { css.lowercased().contains($0) }) {
            throw EngineFailure(message: "Extension styles cannot load remote resources or contain CSS escapes.")
        }
    }
}

struct InstalledExtension: Codable, Identifiable {
    let package: QuillExtension
    let source: String
    let sha256: String
    var enabled: Bool
    var id: String { package.id }
}

final class NoExtensionRedirects: NSObject, URLSessionTaskDelegate {
    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse,
                    newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void) { completionHandler(nil) }
}

@MainActor
final class ExtensionManager: ObservableObject {
    @Published var installed: [InstalledExtension] = []
    @Published var preview: InstalledExtension?
    @Published var busy = false
    @Published var error: String?
    private let file: URL
    private var compiled: [String: WKContentRuleList] = [:]
    private var preparation: Task<Void, Never>?
    private(set) var generation = 0
    private var preparedGeneration = -1
    private var preparationGeneration = -1

    init(directory: URL? = nil) {
        let directory = directory ?? FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appendingPathComponent("Quill Search")
        file = directory.appendingPathComponent("extensions.json")
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            if FileManager.default.fileExists(atPath: file.path) {
                let data = try Data(contentsOf: file)
                guard data.count <= 8_000_000 else { throw EngineFailure(message: "Extension storage is too large.") }
                installed = try JSONDecoder().decode([InstalledExtension].self, from: data)
                guard installed.count <= 20, Set(installed.map(\.id)).count == installed.count else { throw EngineFailure(message: "Invalid extension registry.") }
                for item in installed { try item.package.validate() }
            }
        } catch { installed = []; self.error = error.localizedDescription }
    }

    static func githubURL(_ text: String) throws -> URL {
        guard let url = URL(string: text.trimmingCharacters(in: .whitespacesAndNewlines)),
              url.scheme == "https", url.host == "raw.githubusercontent.com", url.port == nil,
              url.user == nil, url.password == nil, url.query == nil, url.fragment == nil,
              url.pathComponents.count >= 5, url.path.hasSuffix(".json") else {
            throw EngineFailure(message: "Use a raw.githubusercontent.com URL to a Quill extension JSON package.")
        }
        return url
    }

    func fetch(_ text: String) async {
        guard !busy else { return }
        busy = true; error = nil; preview = nil
        defer { busy = false }
        do {
            let url = try Self.githubURL(text)
            let configuration = URLSessionConfiguration.ephemeral
            configuration.timeoutIntervalForRequest = 20
            configuration.timeoutIntervalForResource = 30
            let session = URLSession(configuration: configuration, delegate: NoExtensionRedirects(), delegateQueue: nil)
            defer { session.invalidateAndCancel() }
            let (bytes, response) = try await session.bytes(from: url)
            guard (response as? HTTPURLResponse)?.statusCode == 200 else { throw EngineFailure(message: "GitHub did not return the package (HTTP \((response as? HTTPURLResponse)?.statusCode ?? 0)).") }
            var data = Data()
            for try await byte in bytes {
                guard data.count < 1_000_000 else { throw EngineFailure(message: "Extension exceeds the 1 MB download limit.") }
                data.append(byte)
            }
            let package = try JSONDecoder().decode(QuillExtension.self, from: data)
            try package.validate()
            if let rules = package.rules { _ = try await compile(package.id, rules) }
            preview = InstalledExtension(package: package, source: url.absoluteString,
                sha256: SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined(), enabled: true)
        } catch { self.error = error.localizedDescription }
    }

    private func compile(_ id: String, _ rules: String) async throws -> WKContentRuleList {
        let digest = SHA256.hash(data: Data(rules.utf8)).map { String(format: "%02x", $0) }.joined()
        guard let list = try await WKContentRuleListStore.default().compileContentRuleList(forIdentifier: "quill-\(id)-\(digest)", encodedContentRuleList: rules) else {
            throw EngineFailure(message: "WebKit did not compile the content rules.")
        }
        return list
    }

    private func save(_ items: [InstalledExtension]) throws {
        guard items.count <= 20 else { throw EngineFailure(message: "Up to 20 extensions can be installed.") }
        let data = try JSONEncoder().encode(items)
        guard data.count <= 8_000_000 else { throw EngineFailure(message: "Extension storage limit reached.") }
        try data.write(to: file, options: .atomic)
        installed = items; generation += 1; error = nil
    }

    func installPreview() {
        guard let preview, !busy else { return }
        do {
            var items = installed.filter { $0.id != preview.id }
            items.append(preview); try save(items); self.preview = nil
        } catch { self.error = error.localizedDescription }
    }

    func setEnabled(_ id: String, _ value: Bool) {
        var items = installed
        guard let i = items.firstIndex(where: { $0.id == id }) else { return }
        items[i].enabled = value
        do { try save(items) } catch { self.error = error.localizedDescription }
    }

    func remove(_ id: String) {
        do { try save(installed.filter { $0.id != id }) } catch { self.error = error.localizedDescription }
    }

    func apply(to controller: WKUserContentController) async {
        let requestedGeneration = generation
        if preparedGeneration != generation {
            if preparationGeneration != generation {
                let revision = generation
                let items = installed.filter(\.enabled)
                preparationGeneration = revision
                preparation = Task {
                    var lists: [String: WKContentRuleList] = [:]
                    do {
                        for item in items {
                            if let rules = item.package.rules { lists[item.id] = try await compile(item.id, rules) }
                        }
                        if generation == revision { compiled = lists; preparedGeneration = revision }
                    } catch { self.error = "Extension rules could not be loaded: \(error.localizedDescription)" }
                }
            }
            await preparation?.value
        }
        if requestedGeneration != generation { await apply(to: controller); return }
        controller.removeAllContentRuleLists()
        controller.removeAllUserScripts()
        for item in installed where item.enabled {
            if let rules = compiled[item.id] { controller.add(rules) }
            if let css = item.package.css,
               let encoded = try? JSONSerialization.data(withJSONObject: [css]),
               let literal = String(data: encoded, encoding: .utf8) {
                let script = "if (/^https?:$/.test(location.protocol)) { const s=document.createElement('style'); s.textContent=(\(literal))[0]; (document.head || document.documentElement).appendChild(s); }"
                controller.addUserScript(WKUserScript(source: script, injectionTime: .atDocumentEnd, forMainFrameOnly: true, in: .defaultClient))
            }
        }
    }
}

struct ExtensionsView: View {
    @ObservedObject var manager: ExtensionManager
    let changed: () -> Void
    @Environment(\.dismiss) private var dismiss
    @State private var address = ""
    private let base = "https://raw.githubusercontent.com/Benji-Bitter/Quill/main/extensions/quill/"
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack { Label("Extensions", systemImage: "puzzlepiece.extension").font(.title.bold()); Spacer(); Button("Done") { dismiss() } }
            Text("Install Quill packages from GitHub. Changes apply when you next open or reload a page.").foregroundStyle(.secondary)
            HStack {
                TextField("Raw GitHub package URL", text: $address)
                Button("Preview") { Task { await manager.fetch(address) } }.disabled(manager.busy)
            }
            HStack {
                ForEach([("Ad blocker", "ad-blocker"), ("Dark pages", "dark-pages"), ("Reduced motion", "reduced-motion")], id: \.1) { title, id in
                    Button(title) { address = base + id + ".json"; Task { await manager.fetch(address) } }.disabled(manager.busy)
                }
                if manager.busy { ProgressView().controlSize(.small) }
            }
            if let error = manager.error { Text(error).foregroundStyle(.red).textSelection(.enabled) }
            if let preview = manager.preview {
                VStack(alignment: .leading, spacing: 8) {
                    Text("\(preview.package.name) · \(preview.package.version)").font(.headline)
                    Text(preview.package.description)
                    Text([preview.package.rules != nil ? "Block matching network requests" : nil, preview.package.css != nil ? "Change website appearance on all pages" : nil].compactMap { $0 }.joined(separator: " · ")).font(.caption)
                    Text(preview.source).font(.caption).textSelection(.enabled)
                    Button(manager.installed.contains(where: { $0.id == preview.id }) ? "Replace installed version" : "Install") { manager.installPreview(); changed() }
                }.padding().background(.quaternary).clipShape(RoundedRectangle(cornerRadius: 12))
            }
            Divider()
            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    if manager.installed.isEmpty { Text("No extensions installed yet.").foregroundStyle(.secondary) }
                    ForEach(manager.installed) { item in
                        VStack(alignment: .leading, spacing: 6) {
                            HStack {
                                Toggle(item.package.name, isOn: Binding(get: { item.enabled }, set: { manager.setEnabled(item.id, $0); changed() }))
                                Spacer()
                                Button("Update…") { address = item.source; Task { await manager.fetch(item.source) } }.disabled(manager.busy)
                                Button("Remove", role: .destructive) { manager.remove(item.id); changed() }
                            }
                            Text(item.package.description).font(.caption).foregroundStyle(.secondary)
                            Text("Version \(item.package.version) · SHA-256 \(item.sha256.prefix(12))…").font(.caption2).textSelection(.enabled)
                        }
                    }
                }.frame(maxWidth: .infinity, alignment: .leading)
            }
            Text("Quill format: content-blocking rules and styles. Chrome/Firefox extension packages and remote JavaScript are not supported. Updates are manual.").font(.caption).foregroundStyle(.secondary)
        }.padding(26).frame(width: 660, height: 600)
    }
}
