import AppKit
import SwiftUI

struct SearchHit: Decodable, Identifiable {
    let title: String
    let url: String
    let snippet: String
    let score: Double
    let matched_terms: [String]
    let repetition_factor: Double
    var id: String { url }
}

struct IndexStats: Decodable {
    var documents = 0
    var origins: [String: Int] = [:]
}

struct CrawlReport: Decodable {
    let seed: String
    let added: Int?
    let fetched: Int?
    let denied: Int?
    let skipped: Int?
    let error: String?
}

struct EngineFailure: LocalizedError {
    let message: String
    var errorDescription: String? { message }
}

@MainActor
final class SearchModel: ObservableObject {
    @Published var query = ""
    @Published var searchedQuery = ""
    @Published var hits: [SearchHit] = []
    @Published var stats = IndexStats()
    @Published var searching = false
    @Published var crawling = false
    @Published var status = ""
    @Published var crawlLog: [String] = []
    @Published var error: String?
    @Published var elapsed: Double = 0
    private var crawlProcess: Process?
    private var cancelled = false
    private var searchVersion = 0
    let directory: URL
    var index: URL { directory.appendingPathComponent("web-index.json") }
    let executable: URL

    init(storage: URL? = nil, engine: URL? = nil) {
        executable = engine ?? Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/quill-search")
        directory = storage ?? FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Quill Search", isDirectory: true)
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true,
                                                   attributes: [.posixPermissions: 0o700])
        } catch { self.error = error.localizedDescription }
        Task { await refresh() }
    }

    private func run(_ arguments: [String], isCrawl: Bool = false) async throws -> Data {
        let process = Process()
        process.executableURL = executable
        process.arguments = arguments
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe
        try process.run()
        if isCrawl { crawlProcess = process }
        let result: (Data, Int32) = await withCheckedContinuation { continuation in
            DispatchQueue.global(qos: .userInitiated).async {
                let output = pipe.fileHandleForReading.readDataToEndOfFile()
                process.waitUntilExit()
                continuation.resume(returning: (output, process.terminationStatus))
            }
        }
        if isCrawl { crawlProcess = nil }
        guard result.1 == 0 else {
            throw EngineFailure(message: String(data: result.0, encoding: .utf8) ?? "Search engine exited unexpectedly.")
        }
        return result.0
    }

    func refresh() async {
        do { stats = try JSONDecoder().decode(IndexStats.self, from: await run(["stats", index.path])) }
        catch { self.error = error.localizedDescription }
    }

    func search() {
        searchVersion += 1
        let version = searchVersion
        let text = query.trimmingCharacters(in: .whitespacesAndNewlines)
        searchedQuery = text
        guard !text.isEmpty else { hits = []; searching = false; return }
        guard stats.documents > 0 else { hits = []; searching = false; return }
        searching = true
        error = nil
        Task {
            let start = Date()
            do {
                let data = try await run(["query", index.path, text])
                let results = try JSONDecoder().decode([SearchHit].self, from: data)
                guard version == searchVersion else { return }
                hits = results
                elapsed = Date().timeIntervalSince(start) * 1000
            } catch {
                guard version == searchVersion else { return }
                self.error = error.localizedDescription
                hits = []
            }
            searching = false
        }
    }

    func collect(_ text: String, pages: Int) {
        guard !crawling else { return }
        let seeds = text.split(whereSeparator: { $0.isWhitespace }).map(String.init)
        guard !seeds.isEmpty, seeds.count <= 20 else {
            error = "Enter between 1 and 20 website URLs."; return
        }
        for seed in seeds {
            guard let url = URL(string: seed), ["http", "https"].contains(url.scheme?.lowercased() ?? ""),
                  url.host != nil, url.user == nil, url.password == nil else {
                error = "Enter complete http:// or https:// URLs without credentials."; return
            }
        }
        crawling = true
        cancelled = false
        error = nil
        crawlLog = []
        Task {
            for (offset, seed) in seeds.enumerated() {
                if cancelled { break }
                status = "Crawling \(offset + 1) of \(seeds.count) · \(seed)"
                do {
                    let data = try await run(["collect", index.path, String(pages), seed], isCrawl: true)
                    let report = try JSONDecoder().decode(CrawlReport.self, from: data)
                    if let failure = report.error {
                        crawlLog.append("\(seed) — \(failure)")
                    } else {
                        crawlLog.append("\(seed) — \(report.added ?? 0) added, \(report.fetched ?? 0) fetched, \(report.denied ?? 0) robots exclusions, \(report.skipped ?? 0) skipped")
                    }
                    await refresh()
                } catch {
                    if !cancelled { crawlLog.append("\(seed) — \(error.localizedDescription)") }
                }
            }
            status = cancelled ? "Crawl stopped. Completed sites are saved." : "Crawl complete · \(stats.documents) searchable pages"
            crawling = false
            if !searchedQuery.isEmpty { search() }
        }
    }

    func stop() {
        cancelled = true
        crawlProcess?.terminate()
    }

    func exportIndex() {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "quill-web-index.json"
        guard panel.runModal() == .OK, let destination = panel.url else { return }
        do { try Data(contentsOf: index).write(to: destination, options: .atomic) }
        catch { self.error = error.localizedDescription }
    }

    func deleteIndex() {
        guard !crawling else { return }
        searchVersion += 1
        do {
            if FileManager.default.fileExists(atPath: index.path) { try FileManager.default.removeItem(at: index) }
            stats = IndexStats(); hits = []; searchedQuery = ""; query = ""; status = "Index deleted"; searching = false
        } catch { self.error = error.localizedDescription }
    }
}
