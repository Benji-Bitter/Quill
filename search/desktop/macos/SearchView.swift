import AppKit
import SwiftUI

struct Brand: View {
    @Environment(\.colorScheme) var scheme
    var body: some View {
        HStack(spacing: 12) {
            if let image = NSImage(contentsOf: Bundle.main.resourceURL!.appendingPathComponent(scheme == .dark ? "white icon.png" : "black icon.png")) {
                Image(nsImage: image).resizable().scaledToFit().frame(width: 220, height: 220)
                    .frame(width: 38, height: 38).clipped().accessibilityHidden(true)
            }
            Text("Quill").font(.system(size: 27, weight: .semibold, design: .serif))
        }
    }
}

struct SearchView: View {
    @ObservedObject var model: SearchModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var showSources = false
    @State private var showRanking = false
    @FocusState private var queryFocused: Bool
    @Environment(\.colorScheme) var scheme
    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Brand()
                Spacer()
                Label("On your Mac", systemImage: "internaldrive").font(.system(size: 12)).foregroundStyle(.secondary)
                Button("Sources", systemImage: "square.stack") { showSources = true }.padding(.leading, 18)
                Button("Ranking", systemImage: "slider.horizontal.3") { showRanking = true }
            }.buttonStyle(.plain).padding(.horizontal, 36).padding(.vertical, 22)
            Divider()
            ScrollView {
                VStack(alignment: .leading, spacing: 24) {
                    if model.searchedQuery.isEmpty {
                        VStack(alignment: .leading, spacing: 12) {
                            Text("Your web.\nA little clearer.")
                                .font(.system(size: 44, weight: .regular, design: .serif))
                            Text("Real pages. Your own index. No ads or query uploads.")
                                .font(.system(size: 15)).foregroundStyle(.secondary)
                        }.padding(.top, 52).padding(.bottom, 14)
                    }
                    HStack(spacing: 14) {
                        Image(systemName: "magnifyingglass").foregroundStyle(.secondary)
                        TextField("Search your web index", text: $model.query)
                            .textFieldStyle(.plain).font(.system(size: 18))
                            .focused($queryFocused).onSubmit { model.search() }
                            .accessibilityIdentifier("search-field")
                        if model.searching { ProgressView().controlSize(.small) }
                        Button(action: { model.search() }) {
                            Image(systemName: "arrow.right").padding(9)
                        }.buttonStyle(.plain).accessibilityLabel("Search")
                    }.padding(14)
                        .background(scheme == .dark ? Color.white.opacity(0.06) : Color.white)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                        .overlay(RoundedRectangle(cornerRadius: 12).stroke(queryFocused ? Color.teal.opacity(0.7) : Color.primary.opacity(0.12), lineWidth: 1))
                        .shadow(color: .black.opacity(0.04), radius: 14, y: 5)
                        .animation(reduceMotion ? nil : .easeOut(duration: 0.18), value: queryFocused)
                    if let error = model.error {
                        Text(error).foregroundStyle(.red).textSelection(.enabled)
                    }
                    if model.crawling {
                        HStack { ProgressView().controlSize(.small); Text(model.status).lineLimit(2); Spacer(); Button("Stop", action: model.stop) }
                            .font(.system(size: 12)).foregroundStyle(.secondary)
                    }
                    if model.stats.documents == 0 && model.hits.isEmpty {
                        VStack(alignment: .leading, spacing: 14) {
                            Text("Start with the websites you want to search.").font(.system(size: 21, design: .serif))
                            Text("Quill fetches public pages directly, follows their links and builds a searchable index here. Add sources to get started.")
                                .foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                            Button("Add websites") { showSources = true }.controlSize(.large)
                        }.padding(.vertical, 12)
                    } else if !model.searchedQuery.isEmpty && !model.searching {
                        Text("\(model.hits.count) results · \(model.stats.documents) indexed pages · \(Int(model.elapsed)) ms")
                            .font(.system(size: 12)).foregroundStyle(.secondary)
                        if model.hits.isEmpty {
                            VStack(alignment: .leading, spacing: 10) {
                                Text("No matching pages in your index.").font(.system(size: 22, design: .serif))
                                Text("Try different words or crawl more websites. You can also search for a saved website by name.").foregroundStyle(.secondary)
                                Button("Add more sources") { showSources = true }
                            }
                        }
                        LazyVStack(alignment: .leading, spacing: 28) {
                            ForEach(model.hits) { hit in ResultRow(hit: hit).transition(.opacity) }
                        }
                    } else if model.stats.documents > 0 && model.searchedQuery.isEmpty {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("\(model.stats.documents) pages from \(model.stats.origins.count) websites")
                                .font(.system(size: 13, weight: .medium))
                            Text("Search works offline. Open a result to read its source inside Quill.")
                                .font(.system(size: 12)).foregroundStyle(.secondary)
                        }
                    }
                    Spacer(minLength: 40)
                }.frame(maxWidth: 760, alignment: .leading).padding(.horizontal, 36).padding(.top, 30).frame(maxWidth: .infinity)
            }
            Divider()
            HStack {
                Text("Independent index · Local search")
                Spacer()
                Text("Coverage is limited to the websites you crawl.")
            }.font(.system(size: 11)).foregroundStyle(.secondary).padding(.horizontal, 36).padding(.vertical, 13)
        }
        .background(scheme == .dark ? Color(red: 0.09, green: 0.09, blue: 0.085) : Color(red: 0.975, green: 0.97, blue: 0.95))
        .sheet(isPresented: $showSources) { SourcesView(model: model) }
        .sheet(isPresented: $showRanking) { RankingView() }
        .animation(reduceMotion ? nil : .easeOut(duration: 0.18), value: model.hits.map(\.id))
        .onAppear { queryFocused = true }
    }
}

struct ResultRow: View {
    let hit: SearchHit
    @State private var hovered = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(hit.url).font(.system(size: 11)).foregroundStyle(.secondary).lineLimit(1).textSelection(.enabled)
            if let url = URL(string: hit.url), ["https", "http"].contains(url.scheme ?? "") {
                Link(destination: url) {
                    Text(hit.title.isEmpty ? hit.url : hit.title)
                        .font(.system(size: 23, weight: .medium, design: .serif))
                        .foregroundStyle(.primary).multilineTextAlignment(.leading)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }.buttonStyle(.plain)
            }
            Text(hit.snippet).font(.system(size: 14)).lineSpacing(4).foregroundStyle(.secondary).textSelection(.enabled)
            if hit.website == true {
                Text("Saved website · Homepage shortcut").font(.system(size: 11)).foregroundStyle(.secondary)
            } else {
            DisclosureGroup("Source details & ranking") {
                if let fetched = hit.fetched_at {
                    Text("Fetched: \(Date(timeIntervalSince1970: fetched).formatted(date: .abbreviated, time: .shortened))")
                        .font(.system(size: 12)).foregroundStyle(.secondary)
                } else {
                    Text("Crawl date unavailable for this older entry.").font(.system(size: 12)).foregroundStyle(.secondary)
                }
                if let published = hit.published_at {
                    Text("Publisher-supplied date: \(published)").font(.system(size: 12)).foregroundStyle(.secondary)
                }
                Text("Content has not been independently fact-checked by Quill.")
                    .font(.system(size: 12)).foregroundStyle(.secondary)

                Text("Matched: \(hit.matched_terms.joined(separator: ", "))\nBM25 score: \(hit.score.formatted(.number.precision(.fractionLength(3)))) · repetition factor: \(hit.repetition_factor.formatted(.number.precision(.fractionLength(2))))\nTitle matches have 3× term weight. Scores measure text relevance, not truth.")
                    .font(.system(size: 12)).foregroundStyle(.secondary).frame(maxWidth: .infinity, alignment: .leading).padding(.top, 6)
            }.font(.system(size: 11)).foregroundStyle(.secondary)
            }
        }
        .padding(20)
        .background(RoundedRectangle(cornerRadius: 16).fill(Color.primary.opacity(hovered ? 0.055 : 0.025)))
        .overlay(RoundedRectangle(cornerRadius: 16).stroke(Color.primary.opacity(hovered ? 0.13 : 0.06)))
        .onHover { hovered = $0 }
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: hovered)
    }
}

struct RankingView: View {
    @Environment(\.dismiss) var dismiss
    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Inspect the ranking.").font(.system(size: 30, design: .serif))
            Text("Results come from Quill’s own crawler and inverted index. Queries stay on your Mac. Exact saved website names or domains show homepage shortcuts; other searches return indexed pages.")
            Text("Quill scores matching words using BM25, with title terms weighted three times and a penalty for extreme repetition. Results use the same rules regardless of publisher; there are no paid placements, political labels or personalized boosts.")
            Text("No ranking is free of bias. Your choice of sources, crawl coverage, language and these scoring rules influence what you see. A high score means a text match, not a verified claim.")
            Text("Current limits: 1,000 pages per index; HTML text only; no JavaScript rendering, semantic matching or worldwide index. Up to 10 results are shown per query.").foregroundStyle(.secondary)
            HStack { Link("Read the source", destination: URL(string: "https://github.com/Benji-Bitter/Quill/tree/main/search")!); Spacer(); Button("Done") { dismiss() }.keyboardShortcut(.defaultAction) }
        }.padding(32).frame(width: 540)
    }
}
