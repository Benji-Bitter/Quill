import AppKit
import SwiftUI

@main
struct SearchAppSmoke {
    @MainActor
    static func main() async throws {
        let args = CommandLine.arguments
        guard args.count >= 3 else { fatalError("usage: smoke ENGINE INDEX [QUERY]") }
        let temporary = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: temporary) }
        let model = SearchModel(storage: temporary, engine: URL(fileURLWithPath: args[1]))
        try FileManager.default.copyItem(at: URL(fileURLWithPath: args[2]), to: model.index)
        await model.refresh()
        precondition(model.stats.documents > 0, "Native model must load the real Rust index")
        model.query = args.count > 3 ? args[3] : "rust ownership"
        model.search()
        while model.searching { try await Task.sleep(nanoseconds: 10_000_000) }
        precondition(model.error == nil, model.error ?? "")
        precondition(!model.hits.isEmpty, "Native model must return actual engine hits")
        precondition(!model.hits[0].matched_terms.isEmpty)
        let firstSearchMilliseconds = model.elapsed
        let cacheStart = Date()
        model.search()
        precondition(!model.searching && !model.hits.isEmpty, "Repeated searches must use the in-memory cache")
        print("Search timing: first \(firstSearchMilliseconds) ms; cached \(Date().timeIntervalSince(cacheStart) * 1000) ms")
        print("Native search: \(model.hits.count) results, first: \(model.hits[0].title)")
        let excludedHost = URL(string: model.hits[0].url)!.host!
        let policy = temporary.appendingPathComponent("excluded-domains.txt")
        try excludedHost.write(to: policy, atomically: true, encoding: .utf8)
        model.search()
        while model.searching { try await Task.sleep(nanoseconds: 10_000_000) }
        precondition(!model.hits.contains { URL(string: $0.url)?.host == excludedHost }, "Changed exclusions must invalidate cached results")
        try FileManager.default.removeItem(at: policy)
        print("Search cache invalidation passed")
        model.query = "quillnonexistentterm92849"
        model.search()
        while model.searching { try await Task.sleep(nanoseconds: 10_000_000) }
        precondition(model.hits.isEmpty, "Unknown words must not return fabricated results")
        model.deleteIndex()
        precondition(model.stats.documents == 0 && !FileManager.default.fileExists(atPath: model.index.path))
        await model.refresh()
        precondition(model.stats.documents == 0)
        print("Native empty results and index deletion passed")
        try "https://www.youtube.com/creators\nhttps://youtube.com/\nhttps://kick.com/\n".write(to: temporary.appendingPathComponent("sources.txt"), atomically: true, encoding: .utf8)
        model.sourceNotes = [SourceNote(name: "YouTube", url: "https://youtube.com/", category: "Video", source_information: "https://youtube.com/")]
        for term in ["YouTube", "youtube.com", "www.youtube.com"] {
            model.query = term; model.search()
            precondition(model.hits.count == 1 && model.hits[0].url == "https://youtube.com/")
            precondition(model.hits[0].website == true && model.hits[0].fetched_at == nil)
        }
        precondition(model.websiteResults(for: "youtube creators").isEmpty, "Topic searches must retain page search")
        precondition(model.websiteResults(for: "notyoutube").isEmpty, "No substring domain matches")
        model.query = "kick"; model.search()
        precondition(model.hits.first?.url == "https://kick.com/", "Saved sites work even without crawled pages")
        try "youtube.com\n".write(to: temporary.appendingPathComponent("excluded-domains.txt"), atomically: true, encoding: .utf8)
        precondition(model.websiteResults(for: "YouTube").isEmpty, "Website shortcuts respect exclusions")
        try FileManager.default.removeItem(at: temporary.appendingPathComponent("excluded-domains.txt"))
        print("Website homepage search, deduplication, topic queries and exclusions passed")
        if args.count > 4 {
            model.collect(args[4], pages: 20)
            while model.crawling { try await Task.sleep(nanoseconds: 10_000_000) }
            precondition(model.stats.documents == 3, "Native crawl must persist the three permitted fixture pages: \(model.crawlLog)")
            model.query = "rust ownership"
            model.search()
            while model.searching { try await Task.sleep(nanoseconds: 10_000_000) }
            precondition(model.hits.first?.title == "Rust ownership guide")
            let reopened = SearchModel(storage: temporary, engine: URL(fileURLWithPath: args[1]))
            precondition(reopened.sources == args[4], "Crawl sources must survive restarting the app")
            precondition(!reopened.crawlLog.isEmpty, "Crawl outcomes must survive restarting the app")
            precondition(model.hits.first?.fetched_at != nil, "New crawl results need a fetch timestamp")
            model.excludedDomains = "127.0.0.1"
            precondition(model.saveExclusions())
            await model.refresh()
            precondition(model.stats.documents == 0, "Excluded origins must not appear in stats")
            model.query = "rust ownership"
            model.search()
            while model.searching { try await Task.sleep(nanoseconds: 10_000_000) }
            precondition(model.hits.isEmpty, "Excluded origins must not appear in search")
            print("Native crawl, persistence, provenance and exclusions passed")
        }
    }
}
