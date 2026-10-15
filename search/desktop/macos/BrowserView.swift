import SwiftUI
import WebKit

@MainActor
final class BrowserModel: NSObject, ObservableObject, WKNavigationDelegate, WKUIDelegate {
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

    func returnToSearch() {
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
    @FocusState private var addressFocused: Bool
    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 16) {
                Button("Search", systemImage: "magnifyingglass", action: model.returnToSearch)
                Button { model.webView.goBack() } label: { Image(systemName: "chevron.left") }
                    .disabled(!model.canGoBack).help("Back").accessibilityLabel("Back")
                Button { model.webView.goForward() } label: { Image(systemName: "chevron.right") }
                    .disabled(!model.canGoForward).help("Forward").accessibilityLabel("Forward")
                Button {
                    if model.loading { model.webView.stopLoading() } else { model.webView.reload() }
                } label: { Image(systemName: model.loading ? "xmark" : "arrow.clockwise") }
                    .accessibilityLabel(model.loading ? "Stop loading" : "Reload")
                TextField("Website address", text: $model.address).textFieldStyle(.roundedBorder)
                    .focused($addressFocused).onSubmit { model.navigate() }
                Button("Go", action: model.navigate)
            }.buttonStyle(.plain).padding(14)
            if model.loading { ProgressView().progressViewStyle(.linear).frame(height: 2) }
            if let error = model.error {
                HStack { Text(error).textSelection(.enabled); Spacer(); Button("Retry") { model.navigate() } }
                    .font(.system(size: 12)).padding(12).background(Color.red.opacity(0.08))
            }
            Divider()
            BrowserSurface(model: model)
        }
    }
}

struct QuillWorkspace: View {
    @ObservedObject var search: SearchModel
    @StateObject private var browser = BrowserModel()
    var body: some View {
        Group {
            if browser.visible { BrowserView(model: browser) }
            else { SearchView(model: search) }
        }.environment(\.openURL, OpenURLAction { url in
            browser.open(url)
            return .handled
        })
    }
}
