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
        let canonical=url.resolvingSymlinksInPath().path
        if let previous=held.removeValue(forKey:canonical) {previous.stopAccessingSecurityScopedResource()}
        if url.startAccessingSecurityScopedResource() {held[canonical] = url}
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
        let canonical=url.resolvingSymlinksInPath().path
        if let previous=held.removeValue(forKey:canonical) {previous.stopAccessingSecurityScopedResource()}
        if url.startAccessingSecurityScopedResource() {held[canonical] = url}
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

import NaturalLanguage
private let embeddingLock = NSLock()
@_cdecl("fp_semantic_rank")
public func semanticRank(_ input: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>? {
    embeddingLock.lock()
    defer { embeddingLock.unlock() }
    // No asset download request is made and absence is an explicit fallback
    guard let embedding = NLEmbedding.sentenceEmbedding(for: .english) else {
        return output(["available":false,"reason":"English sentence embedding is not installed on this Mac"])
    }
    let text = String(String(cString:input).prefix(2000))
    guard let query=embedding.vector(for:text) else { return output(["available":false,"reason":"No embedding for this input"]) }
    let descriptions = [
        "Education":"School university college study lessons homework exams textbooks",
        "Development":"Software programming source code technical developer projects",
        "Professional":"Work business employment resumes contracts clients meetings",
        "Finance":"Money payments invoices banking taxes receipts bills accounts",
        "Media":"Photographs images music audio video recordings",
        "Software":"Application installers software packages disk images",
        "Personal":"Personal life family travel diary recipes hobbies"
    ]
    var ranking:[[String:Any]]=[]
    for (category,description) in descriptions {
        guard let vector=embedding.vector(for:description),vector.count==query.count else { continue }
        let numerator=zip(query,vector).reduce(0.0){$0+$1.0*$1.1}
        let denominator=sqrt(query.reduce(0.0){$0+$1*$1}*vector.reduce(0.0){$0+$1*$1})
        let score=denominator>0 ? numerator/denominator : 0
        ranking.append(["category":category,"score":score])
    }
    ranking.sort{($0["score"] as! Double)>($1["score"] as! Double)}
    return output(["available":true,"model":"Apple English sentence embedding","revision":embedding.revision,"ranking":Array(ranking.prefix(3)),"scores_are_probabilities":false,"review_required":true])
}

@_cdecl("fp_confirm_trash")
public func confirmTrash(_ source:UnsafePointer<CChar>)->UnsafeMutablePointer<CChar>? {
    let path=String(cString:source)
    let alert=NSAlert()
    alert.messageText="Move this file to Trash"
    alert.informativeText=path+"\nThis requires separate confirmation in every autonomy mode"
    alert.alertStyle = .warning
    alert.addButton(withTitle:"Cancel")
    alert.addButton(withTitle:"Move to Trash")
    guard alert.runModal() == .alertSecondButtonReturn else {return output(["cancelled":true])}
    do {
        let sourceURL=URL(fileURLWithPath:path)
        let trash=try FileManager.default.url(for:.trashDirectory,in:.userDomainMask,appropriateFor:sourceURL,create:true)
        return output(["confirmed":true,"trash_root":trash.resolvingSymlinksInPath().path])
    } catch {return output(["error":"System Trash is unavailable no file was moved"])}
}
