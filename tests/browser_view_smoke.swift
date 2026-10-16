import AppKit
import WebKit

@main struct BrowserSmoke {
    @MainActor static func wait(_ condition: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(25)
        while !condition() && Date() < deadline { try await Task.sleep(nanoseconds: 50_000_000) }
        precondition(condition(), "Timed out waiting for WebKit")
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
        let browser = BrowserModel()
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
