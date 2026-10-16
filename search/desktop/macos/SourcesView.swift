import SwiftUI

struct SourcesView: View {
    @ObservedObject var model: SearchModel
    @Environment(\.dismiss) var dismiss
    @Environment(\.openURL) var openURL
    @State private var pages = 20
    @State private var confirmDelete = false
    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack {
                Text("Your sources").font(.system(size: 30, design: .serif))
                Spacer()
                Button("Done") { dismiss() }.keyboardShortcut(.cancelAction)
            }
            Text("Add website URLs, one per line. Crawling contacts those sites directly from your Mac. Each crawl stays on its starting website and respects robots exclusions.")
                .font(.system(size: 13)).foregroundStyle(.secondary)
            TextEditor(text: $model.sources).font(.system(size: 13, design: .monospaced))
                .frame(height: 100).padding(8).overlay(RoundedRectangle(cornerRadius: 8).stroke(.secondary.opacity(0.3)))
                .accessibilityLabel("Website URLs")
            HStack {
                Button("Use example sources") {
                    model.sources = "https://science.nasa.gov/\nhttps://www.britannica.com/science/photosynthesis\nhttps://rust-lang.org/learn\nhttps://docs.python.org/3/tutorial/"
                }
                Spacer()
                Picker("Pages per site", selection: $pages) {
                    Text("10").tag(10); Text("20").tag(20); Text("50").tag(50); Text("100").tag(100)
                }.frame(width: 180)
            }.disabled(model.crawling)
            HStack {
                if model.crawling {
                    ProgressView().controlSize(.small)
                    Text(model.status).font(.system(size: 12)).lineLimit(2)
                    Spacer()
                    Button("Stop", action: model.stop)
                } else {
                    Text(model.status).font(.system(size: 12)).foregroundStyle(.secondary)
                    Spacer()
                    Button("Crawl websites") { model.collect(model.sources, pages: pages) }
                        .disabled(model.sources.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            }
            if let error = model.error { Text(error).foregroundStyle(.red).font(.system(size: 12)) }
            DisclosureGroup("Excluded domains") {
                TextEditor(text: $model.excludedDomains).font(.system(size: 12, design: .monospaced))
                    .frame(height: 60).accessibilityLabel("Excluded domains")
                HStack {
                    Text("Applies to crawling and search; does not block browsing.").font(.caption).foregroundStyle(.secondary)
                    Spacer()
                    Button("Save exclusions") { model.saveExclusions() }.disabled(model.crawling)
                }
            }
            Divider()
            HStack {
                Text("\(model.stats.documents) indexed pages").font(.headline)
                Spacer()
                Button("Export index", action: model.exportIndex).disabled(model.stats.documents == 0)
                Button("Delete index", role: .destructive) { confirmDelete = true }
                    .disabled(model.crawling || model.stats.documents == 0)
            }
            ScrollView {
                VStack(alignment: .leading, spacing: 10) {
                    if !model.sourceNotes.isEmpty {
                        DisclosureGroup("Source selection notes") {
                            Text("Categories describe subject matter, not factual accuracy. Check reporting against its evidence and original sources.")
                                .foregroundStyle(.secondary).padding(.vertical, 8)
                            ForEach(model.sourceNotes) { source in
                                VStack(alignment: .leading, spacing: 4) {
                                    HStack {
                                        Text(source.name).fontWeight(.medium)
                                        Spacer()
                                        Button("Source information") {
                                            if let url = URL(string: source.source_information) { dismiss(); openURL(url) }
                                        }
                                    }
                                    Text(source.category).foregroundStyle(.secondary)
                                }.padding(.vertical, 4)
                            }
                        }
                        Divider()
                    }
                    ForEach(model.stats.origins.keys.sorted(), id: \.self) { origin in
                        HStack { Text(origin); Spacer(); Text("\(model.stats.origins[origin] ?? 0) pages").foregroundStyle(.secondary) }
                    }
                    if !model.crawlLog.isEmpty {
                        Divider().padding(.vertical, 6)
                        ForEach(Array(model.crawlLog.enumerated()), id: \.offset) { _, line in
                            Text(line).foregroundStyle(.secondary).textSelection(.enabled)
                        }
                    }
                }.font(.system(size: 12)).frame(maxWidth: .infinity, alignment: .leading)
            }.frame(minHeight: 130)
            Text("Sites may refuse crawling or return pages Quill cannot parse. Your index is stored in Application Support/Quill Search. No crawling occurs unless you start it.")
                .font(.system(size: 11)).foregroundStyle(.secondary)
        }.padding(28).frame(width: 640, height: 620)
            .alert("Delete your entire search index?", isPresented: $confirmDelete) {
                Button("Cancel", role: .cancel) {}
                Button("Delete index", role: .destructive, action: model.deleteIndex)
            } message: { Text("You will need to crawl websites again to search them. Previously exported copies are not deleted.") }
    }
}
