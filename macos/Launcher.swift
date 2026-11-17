// Hollow Hours.app's front door. It starts the game, and it receives hollowhours://login?code=... links from the
// website's account page ("Open Hollow Hours and log in"): the code (checked: 48 hex characters) goes into a file only
// you can read, the game redeems it once with the website, and deletes the file.
import Cocoa

let support = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support/Hollow Hours")

func launchGame() {
    let game = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/hollow-hours-game")
    // Already running? It picks up the login file by itself; just bring it forward.
    let running = NSWorkspace.shared.runningApplications.contains { $0.executableURL?.lastPathComponent == "hollow-hours-game" }
    if running { return }
    let p = Process()
    p.executableURL = game
    p.currentDirectoryURL = support
    try? p.run()
}

final class Delegate: NSObject, NSApplicationDelegate {
    var gotURL = false
    func applicationWillFinishLaunching(_ n: Notification) {
        NSAppleEventManager.shared().setEventHandler(self, andSelector: #selector(handle(_:reply:)),
                                                     forEventClass: AEEventClass(kInternetEventClass), andEventID: AEEventID(kAEGetURL))
    }
    @objc func handle(_ event: NSAppleEventDescriptor, reply: NSAppleEventDescriptor) {
        gotURL = true
        if let s = event.paramDescriptor(forKeyword: keyDirectObject)?.stringValue,
           let u = URLComponents(string: s), u.scheme == "hollowhours", u.host == "login",
           let code = u.queryItems?.first(where: { $0.name == "code" })?.value,
           code.range(of: "^[0-9a-f]{48}$", options: .regularExpression) != nil {
            try? FileManager.default.createDirectory(at: support, withIntermediateDirectories: true)
            let file = support.appendingPathComponent("pending-login")
            FileManager.default.createFile(atPath: file.path, contents: Data(code.utf8), attributes: [.posixPermissions: 0o600])
        }
        launchGame()
        NSApp.terminate(nil)
    }
    func applicationDidFinishLaunching(_ n: Notification) {
        // Opened normally (Dock, Finder): just start the game.
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.4) {
            if !self.gotURL { launchGame(); NSApp.terminate(nil) }
        }
    }
}

let app = NSApplication.shared
let delegate = Delegate()
app.delegate = delegate
app.setActivationPolicy(.prohibited)
app.run()
