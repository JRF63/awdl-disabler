import AppKit
import Cocoa
import OSLog
import ServiceManagement

let bundleID = "awdldisabler.app"
let daemonPlistName = "awdldisabler.app.daemon.plist"

@main
struct AWDLDisablerApp {
    @MainActor static var delegate = AppDelegate()

    static func main() {
        let app = NSApplication.shared
        app.delegate = delegate
        app.setActivationPolicy(.accessory)
        app.run()
    }
}

@MainActor
class AppDelegate: NSObject, NSApplicationDelegate {
    var service: SMAppService!
    var logger: Logger!
    var statusItem: NSStatusItem!

    private func setupStatusItem() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        if let button = statusItem.button {
            button.image = NSImage(
                systemSymbolName: "wifi.slash", accessibilityDescription: "AWDLDisabler")
        }

        let menu = NSMenu()
        menu.addItem(NSMenuItem(title: "Quit", action: #selector(quit), keyEquivalent: "q"))
        statusItem.menu = menu
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        logger = Logger(subsystem: bundleID, category: "app")
        logger.log("Starting launcher")

        setupStatusItem()
        service = SMAppService.daemon(plistName: daemonPlistName)

        do {
            if service.status == .enabled {
                logger.log("Daemon already registered")
            } else {
                try service.register()
            }
        } catch {
            logger.error("Failed to register: \(error)")
            if case .requiresApproval = service.status {
                SMAppService.openSystemSettingsLoginItems()
            }
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        logger.log("Exiting launcher")
    }

    @objc func quit() {
        Task {
            // Better to handle unregister here instead of in applicationWillTerminate
            do {
                try await service.unregister()
            } catch {
                logger.error("Failed to unregister: \(error)")
            }
            // unregister was already await-ed so we can be sure the daemon is terminated
            NSApp.terminate(nil)
        }
    }
}
