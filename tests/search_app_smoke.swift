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
        print("Native search: \(model.hits.count) results, first: \(model.hits[0].title)")
        model.query = "quillnonexistentterm92849"
        model.search()
        while model.searching { try await Task.sleep(nanoseconds: 10_000_000) }
        precondition(model.hits.isEmpty, "Unknown words must not return fabricated results")
        model.deleteIndex()
        precondition(model.stats.documents == 0 && !FileManager.default.fileExists(atPath: model.index.path))
        await model.refresh()
        precondition(model.stats.documents == 0)
        print("Native empty results and index deletion passed")
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
            print("Native crawl → persisted index → ranked results and saved sources passed")
        }
    }
}
