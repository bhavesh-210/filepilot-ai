import Foundation
import AppKit
import ServiceManagement
import Darwin

// All functions are called on the AppKit main thread by the trusted Rust bridge
private var held: [String: URL] = [:]
private func output(_ value: [String: Any]) -> UnsafeMutablePointer<CChar>? {
    let data = try! JSONSerialization.data(withJSONObject: value)
    return strdup(String(data: data, encoding: .utf8)!)
}
@_cdecl("fp_choose_root")
public func chooseRoot() -> UnsafeMutablePointer<CChar>? {
    let panel = NSOpenPanel()
    panel.canChooseFiles = false
    panel.canChooseDirectories = true
    panel.allowsMultipleSelection = false
    panel.message = "Choose a folder FilePilot may organise"
    guard panel.runModal() == .OK, let url = panel.url else { return output(["cancelled": true]) }
    do {
        let bookmark = try url.bookmarkData(options: [.withSecurityScope], includingResourceValuesForKeys: nil, relativeTo: nil)
        _ = url.startAccessingSecurityScopedResource()
        held[url.path] = url
        return output(["path": url.resolvingSymlinksInPath().path, "bookmark": bookmark.base64EncodedString()])
    } catch { return output(["error": "Could not retain folder access"]) }
}
@_cdecl("fp_resolve_root")
public func resolveRoot(_ encoded: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>? {
    guard let data = Data(base64Encoded: String(cString: encoded)) else { return output(["error":"Invalid bookmark"]) }
    do {
        var stale = false
        let url = try URL(resolvingBookmarkData:data, options:[.withSecurityScope,.withoutUI], relativeTo:nil, bookmarkDataIsStale:&stale)
        guard !stale else { return output(["error":"Folder access needs renewal"]) }
        _ = url.startAccessingSecurityScopedResource()
        held[url.path] = url
        return output(["path":url.resolvingSymlinksInPath().path])
    } catch { return output(["error":"Folder access must be renewed"]) }
}
@_cdecl("fp_release_root")
public func releaseRoot(_ path: UnsafePointer<CChar>) {
    let key=String(cString:path)
    if let url=held.removeValue(forKey:key) {url.stopAccessingSecurityScopedResource()}
}
@_cdecl("fp_login")
public func login(_ enabled: Bool) -> UnsafeMutablePointer<CChar>? {
    do {
        if enabled {try SMAppService.mainApp.register()} else {try SMAppService.mainApp.unregister()}
        return output(["enabled":SMAppService.mainApp.status == .enabled])
    } catch {return output(["error":"Login item registration requires an installed app and system approval"])}
}
