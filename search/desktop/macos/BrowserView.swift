import SwiftUI
import WebKit

@MainActor
final class BrowserModel: NSObject, ObservableObject, WKNavigationDelegate, WKUIDelegate {
    @Published var lastPage: URL?
    @Published var visible = false
    @Published var address = ""
    @Published var canGoBack = false
    @Published var canGoForward = false
    @Published var loading = false
    @Published var error: String?
    let webView: WKWebView
    private var observations: [NSKeyValueObservation] = []

    override init() {
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        configuration.preferences.javaScriptCanOpenWindowsAutomatically = false
        webView = WKWebView(frame: .zero, configuration: configuration)
        super.init()
        webView.navigationDelegate = self
        webView.uiDelegate = self
        observations = [
            webView.observe(\.url, options: [.new]) { [weak self] _, _ in
                Task { @MainActor [weak self] in
                    guard let self else { return }
                    self.address = self.webView.url?.absoluteString ?? ""
                }
            },
            webView.observe(\.canGoBack, options: [.new]) { [weak self] _, _ in
                Task { @MainActor [weak self] in
                    guard let self else { return }
                    self.canGoBack = self.webView.canGoBack
                }
            },
            webView.observe(\.canGoForward, options: [.new]) { [weak self] _, _ in
                Task { @MainActor [weak self] in
                    guard let self else { return }
                    self.canGoForward = self.webView.canGoForward
                }
            },
            webView.observe(\.isLoading, options: [.new]) { [weak self] _, _ in
                Task { @MainActor [weak self] in
                    guard let self else { return }
                    self.loading = self.webView.isLoading
                }
            },
        ]
    }

    static func accepts(_ url: URL) -> Bool {
        ["http", "https"].contains(url.scheme?.lowercased() ?? "") && url.host != nil && url.user == nil && url.password == nil
    }

    func open(_ url: URL) {
        guard Self.accepts(url) else { error = "Only HTTP and HTTPS websites can be opened."; return }
        visible = true
        address = url.absoluteString
        error = nil
        webView.load(URLRequest(url: url))
    }

    func navigate() {
        let input = address.trimmingCharacters(in: .whitespacesAndNewlines)
        let text = input.contains("://") ? input : "https://" + input
        guard let url = URL(string: text), Self.accepts(url), !input.contains(where: { $0.isWhitespace }) else {
            error = "Enter a website address, such as https://example.com."; return
        }
        open(url)
    }

    func back() {
        if canGoBack, let previous = webView.backForwardList.backItem?.url, Self.accepts(previous) {
            webView.goBack()
        } else { returnToSearch() }
    }

    func forward() {
        if !visible, let lastPage { open(lastPage) }
        else { webView.goForward() }
    }

    func returnToSearch() {
        if let url = webView.url, Self.accepts(url) { lastPage = url }

        webView.stopLoading()
        // Release page scripts/media rather than leave a hidden website running.
        webView.loadHTMLString("", baseURL: nil)
        visible = false
        error = nil
    }

    func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction,
                 decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
        guard let url = navigationAction.request.url else { decisionHandler(.cancel); return }
        if url.absoluteString == "about:blank" || Self.accepts(url) {
            if navigationAction.targetFrame == nil {
                webView.load(navigationAction.request)
                decisionHandler(.cancel)
            } else { decisionHandler(.allow) }
        } else {
            if navigationAction.targetFrame?.isMainFrame != false {
                error = "This link uses a scheme Quill does not support."
            }
            decisionHandler(.cancel)
        }
    }

    func webView(_ webView: WKWebView, decidePolicyFor navigationResponse: WKNavigationResponse,
                 decisionHandler: @escaping (WKNavigationResponsePolicy) -> Void) {
        if navigationResponse.canShowMIMEType { decisionHandler(.allow) }
        else { error = "This file cannot be displayed. Downloads are not supported yet."; decisionHandler(.cancel) }
    }

    func webView(_ webView: WKWebView, didStartProvisionalNavigation navigation: WKNavigation!) { error = nil }
    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError failure: Error) { show(failure) }
    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError failure: Error) { show(failure) }
    func webViewWebContentProcessDidTerminate(_ webView: WKWebView) { error = "The page process stopped. Reload to try again." }
    private func show(_ failure: Error) {
        if (failure as NSError).code != NSURLErrorCancelled { error = failure.localizedDescription }
    }
    func webView(_ webView: WKWebView, requestMediaCapturePermissionFor origin: WKSecurityOrigin,
                 initiatedByFrame frame: WKFrameInfo, type: WKMediaCaptureType,
                 decisionHandler: @escaping (WKPermissionDecision) -> Void) { decisionHandler(.deny) }
}

struct BrowserSurface: NSViewRepresentable {
    let model: BrowserModel
    func makeNSView(context: Context) -> WKWebView { model.webView }
    func updateNSView(_ view: WKWebView, context: Context) {}
}

struct BrowserView: View {
    @ObservedObject var model: BrowserModel
    var body: some View {
        VStack(spacing: 0) {
            if model.loading { ProgressView().progressViewStyle(.linear).frame(height: 2) }
            if let error = model.error {
                HStack { Text(error).textSelection(.enabled); Spacer(); Button("Retry") { model.navigate() } }
                    .font(.system(size: 12)).padding(12).background(Color.red.opacity(0.08))
            }
            BrowserSurface(model: model)
        }
    }
}

struct QuillWorkspace: View {
    @ObservedObject var search: SearchModel
    @StateObject private var browser = BrowserModel()
    @Environment(\.colorScheme) private var scheme
    @State private var location = ""
    @FocusState private var locationFocused: Bool

    private func submit() {
        let input = location.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !input.isEmpty else { return }
        let looksLikeAddress = !input.contains(where: { $0.isWhitespace }) &&
            (input.contains("://") || input.contains(".") || input.hasPrefix("localhost"))
        if looksLikeAddress {
            browser.address = input
            browser.navigate()
        } else {
            browser.returnToSearch()
            search.query = input
            search.search()
        }
        locationFocused = false
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 12) {
                Button(action: browser.back) { Label("Back", systemImage: "chevron.left") }
                    .disabled(!browser.visible).keyboardShortcut("[", modifiers: .command)
                Button(action: browser.forward) { Image(systemName: "chevron.right") }
                    .disabled(browser.visible ? !browser.canGoForward : browser.lastPage == nil)
                    .accessibilityLabel(browser.visible ? "Forward" : "Reopen last page")
                    .help(browser.visible ? "Forward (⌘])" : "Reopen last page (⌘])")
                    .keyboardShortcut("]", modifiers: .command)
                Button("Results", systemImage: "house", action: browser.returnToSearch)
                    .disabled(!browser.visible).keyboardShortcut("h", modifiers: [.command, .shift])
                HStack(spacing: 8) {
                    Button {
                        locationFocused = true
                        DispatchQueue.main.async { NSApp.sendAction(#selector(NSText.selectAll(_:)), to: nil, from: nil) }
                    } label: { Image(systemName: "magnifyingglass") }
                        .keyboardShortcut("l", modifiers: .command).help("Focus address or search (⌘L)")
                    TextField("Search your index or enter a website", text: $location)
                        .textFieldStyle(.plain).focused($locationFocused).onSubmit(submit)
                        .accessibilityLabel("Search or website address")
                    Button(action: submit) { Image(systemName: "arrow.right.circle") }.accessibilityLabel("Go")
                }.padding(10).background(scheme == .dark ? Color(red: 0.15, green: 0.15, blue: 0.145) : Color.white).clipShape(RoundedRectangle(cornerRadius: 9))
                Button {
                    if browser.loading { browser.webView.stopLoading() }
                    else { browser.webView.reload() }
                } label: { Image(systemName: browser.loading ? "xmark" : "arrow.clockwise") }
                    .disabled(!browser.visible).keyboardShortcut("r", modifiers: .command)
                    .accessibilityLabel(browser.loading ? "Stop" : "Reload").help("Reload (⌘R)")
            }.buttonStyle(.borderless).padding(.horizontal, 18).padding(.vertical, 10)
                .foregroundStyle(scheme == .dark ? Color.white : Color.black)
                .background(scheme == .dark ? Color(red: 0.09, green: 0.09, blue: 0.085) : Color(red: 0.975, green: 0.97, blue: 0.95))
            Divider()
            if browser.visible { BrowserView(model: browser) }
            else { SearchView(model: search) }
        }
        .onChange(of: browser.address) { _, value in
            if browser.visible && !locationFocused { location = value }
        }
        .onChange(of: browser.visible) { _, visible in
            if !visible { location = search.query }
        }
        .environment(\.openURL, OpenURLAction { url in
            locationFocused = false
            browser.open(url)
            return .handled
        })
    }
}
