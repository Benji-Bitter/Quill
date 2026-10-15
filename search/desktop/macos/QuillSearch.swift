import AppKit
import SwiftUI

@main
struct QuillSearchApp: App {
    @StateObject private var model = SearchModel()
    var body: some Scene {
        WindowGroup("Quill Search") {
            QuillWorkspace(search: model)
                .frame(minWidth: 760, minHeight: 560)
                .onAppear { NSApp.activate(ignoringOtherApps: true) }
                .onReceive(NotificationCenter.default.publisher(for: NSApplication.willTerminateNotification)) { _ in model.stop() }
        }
        .defaultSize(width: 1060, height: 780)
        .commands {
            CommandGroup(replacing: .newItem) {}
        }
    }
}
