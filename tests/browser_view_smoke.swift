import AppKit
import WebKit

@main struct BrowserSmoke {
    @MainActor static func wait(_ condition: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(25)
        while !condition() && Date() < deadline { try await Task.sleep(nanoseconds: 50_000_000) }
        precondition(condition(), "Timed out waiting for WebKit")
    }
    @MainActor static func javascript(_ browser: BrowserModel, _ source: String) async throws -> Any {
        try await withCheckedThrowingContinuation { continuation in
            browser.webView.callAsyncJavaScript(source, arguments: [:], in: nil, in: .page) { result in
                continuation.resume(with: result)
            }
        }
    }
    @MainActor static func main() {
        setbuf(stdout, nil)
        let app = NSApplication.shared
        Task { @MainActor in
            do { try await run(); exit(0) }
            catch { fatalError(error.localizedDescription) }
        }
        app.run()
    }
    @MainActor static func run() async throws {
        let extensionDirectory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: extensionDirectory) }
        let store = WKWebsiteDataStore(forIdentifier: UUID())
        let browser = BrowserModel(dataStore: store, extensionDirectory: extensionDirectory)
        precondition(browser.webView.configuration.websiteDataStore.isPersistent)
        let cookie = HTTPCookie(properties: [.domain: "127.0.0.1", .path: "/", .name: "quill-test", .value: "saved", .expires: Date().addingTimeInterval(3600)])!
        await store.httpCookieStore.setCookie(cookie)
        let reopened = BrowserModel(dataStore: store, extensionDirectory: extensionDirectory)
        let cookies = await reopened.webView.configuration.websiteDataStore.httpCookieStore.allCookies()
        precondition(cookies.contains { $0.name == "quill-test" && $0.value == "saved" })
        await reopened.clearWebsiteData()
        let cleared = await store.httpCookieStore.allCookies()
        precondition(cleared.isEmpty)
        print("Persistent cookie store, shared browser cookies and clear website data passed")
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 800, height: 600), styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = browser.webView
        window.orderBack(nil)
        defer { window.orderOut(nil) }
        let base = CommandLine.arguments[1]
        precondition(!BrowserModel.accepts(URL(string: "file:///etc/passwd")!))
        precondition(!BrowserModel.accepts(URL(string: "javascript:alert(1)")!))
        precondition(!BrowserModel.accepts(URL(string: "https://user:password@example.com")!))
        browser.open(URL(string: base + "/one")!)
        try await wait {
            if let error = browser.error { fatalError("WebKit load: \(error)") }
            return browser.webView.title == "One" && !browser.loading
        }
        print("First page loaded")
        for bad in ["http://raw.githubusercontent.com/a/b/main/a.json", "https://evil.test/a.json", "https://raw.githubusercontent.com@localhost/a/b/main/a.json"] {
            do { _ = try ExtensionManager.githubURL(bad); fatalError("Unsafe package URL accepted") } catch {}
        }
        let manager = browser.extensions
        let fixture = QuillExtension(id: "test-blocker", name: "Test blocker", version: "1", description: "Local fixture", rules: #"[{"trigger":{"url-filter":"/ad[.]js"},"action":{"type":"block"}}]"#, css: "body { background-color: rgb(20, 23, 25) !important; }")
        try fixture.validate()
        manager.preview = InstalledExtension(package: fixture, source: "https://raw.githubusercontent.com/test/test/main/test.json", sha256: "fixture", enabled: true)
        manager.installPreview()
        await manager.apply(to: browser.webView.configuration.userContentController)
        precondition(manager.error == nil, manager.error ?? "")
        browser.open(URL(string: base + "/one?extensions")!)
        try await wait { browser.webView.url?.query == "extensions" && !browser.webView.isLoading }
        let blocked = try await javascript(browser, "try { await fetch('/ad.js'); return false; } catch { return true; }") as? Bool
        precondition(blocked == true, "Enabled extension must block the request")
        let background = try await browser.webView.evaluateJavaScript("getComputedStyle(document.body).backgroundColor") as? String
        precondition(background == "rgb(20, 23, 25)", "Extension CSS must affect the page")
        precondition(ExtensionManager(directory: extensionDirectory).installed.count == 1, "Extensions must persist")
        manager.setEnabled("test-blocker", false)
        await manager.apply(to: browser.webView.configuration.userContentController)
        let allowed = try await javascript(browser, "try { await fetch('/ad.js'); return true; } catch { return false; }") as? Bool
        precondition(allowed == true, "Disabled extension must permit the request")
        manager.remove("test-blocker")
        precondition(ExtensionManager(directory: extensionDirectory).installed.isEmpty)
        for name in ["ad-blocker", "dark-pages", "reduced-motion"] {
            let data = try Data(contentsOf: URL(fileURLWithPath: FileManager.default.currentDirectoryPath + "/extensions/quill/" + name + ".json"))
            let package = try JSONDecoder().decode(QuillExtension.self, from: data)
            try package.validate()
            manager.preview = InstalledExtension(package: package, source: "fixture", sha256: "fixture", enabled: true)
            manager.installPreview()
        }
        await manager.apply(to: browser.webView.configuration.userContentController)
        precondition(manager.error == nil, manager.error ?? "")
        for item in manager.installed { manager.remove(item.id) }
        await manager.apply(to: browser.webView.configuration.userContentController)
        browser.open(URL(string: base + "/one")!)
        try await wait { browser.webView.url?.query == nil && !browser.webView.isLoading }
        print("Extension blocking, styles, toggle, persistence, removal and starter compilation passed")
        let body = try await browser.webView.evaluateJavaScript("document.body.textContent") as? String
        precondition(body?.contains("Real website content") == true)
        // A scripted click has no user gesture; allow it only in this test.
        browser.webView.configuration.preferences.javaScriptCanOpenWindowsAutomatically = true
        try await browser.webView.evaluateJavaScript("document.querySelector('a').click()")
        try await wait { browser.webView.title == "Two" && !browser.loading }
        print("Target blank loaded")
        precondition(browser.address == base + "/two" && browser.canGoBack)
        browser.webView.goBack()
        try await wait { browser.webView.title == "One" && !browser.loading }
        print("Back passed")
        precondition(browser.canGoForward)
        browser.webView.goForward()
        try await wait { browser.webView.title == "Two" && !browser.loading }
        print("Forward passed")
        let beforeReload = try await browser.webView.evaluateJavaScript("document.body.dataset.load") as? String
        browser.webView.reload()
        try await wait { !browser.webView.isLoading }
        let afterReload = try await browser.webView.evaluateJavaScript("document.body.dataset.load") as? String
        precondition(beforeReload != afterReload, "Reload must fetch a new document")
        print("Reload passed")
        browser.returnToSearch()
        precondition(!browser.visible && browser.lastPage?.absoluteString == base + "/two")
        browser.forward()
        try await wait { browser.visible && browser.webView.title == "Two" && !browser.webView.isLoading }
        print("Return to results and reopen last page passed")
        browser.open(URL(string: CommandLine.arguments[2])!)
        try await wait { browser.error != nil }
        browser.returnToSearch()
        precondition(!browser.visible)
        print("Embedded WebKit: content, target-blank link, back, forward, reload, load failure and search return passed")
        if CommandLine.arguments.contains("--https") {
            browser.open(URL(string: "https://example.com")!)
            try await wait { browser.webView.title == "Example Domain" && !browser.loading }
            print("Live HTTPS page loaded inside Quill WebKit")
        }
    }
}
