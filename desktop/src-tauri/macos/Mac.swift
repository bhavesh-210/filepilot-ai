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
@_cdecl("fp_choose_roots")
public func chooseRoots() -> UnsafeMutablePointer<CChar>? {
    let panel = NSOpenPanel()
    panel.canChooseFiles = false
    panel.canChooseDirectories = true
    panel.allowsMultipleSelection = true
    panel.canDownloadUbiquitousContents = false
    panel.canResolveUbiquitousConflicts = false
    panel.prompt = "Allow access"
    panel.message = "Choose the folders FilePilot may maintain Command click to choose several"
    guard panel.runModal() == .OK else { return output(["cancelled":true]) }
    var folders:[[String:String]] = []
    for url in panel.urls {
        do {
            let bookmark = try url.bookmarkData(options:[.withSecurityScope], includingResourceValuesForKeys:nil, relativeTo:nil)
            let canonical = url.resolvingSymlinksInPath().path
            if let previous = held.removeValue(forKey:canonical) { previous.stopAccessingSecurityScopedResource() }
            if url.startAccessingSecurityScopedResource() { held[canonical] = url }
            folders.append(["path":canonical,"bookmark":bookmark.base64EncodedString()])
        } catch {
            // Successful grants remain available and each failed bookmark is reported
            folders.append(["path":url.resolvingSymlinksInPath().path,"bookmark":""])
        }
    }
    return output(["folders":folders])
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

import UserNotifications
@_cdecl("fp_request_notifications")
public func requestReviewNotifications()->UnsafeMutablePointer<CChar>? {
    guard Bundle.main.bundleIdentifier != nil else {return output(["error":"Install the application before requesting notifications"])}
    UNUserNotificationCenter.current().requestAuthorization(options:[.alert,.sound]) { _,_ in }
    return output(["requested":true,"delivery":"Controlled by macOS notification permissions"])
}
@_cdecl("fp_notify_review")
public func notifyReview(_ count:Int32) {
    guard Bundle.main.bundleIdentifier != nil else {return}
    let center=UNUserNotificationCenter.current()
    center.getNotificationSettings { settings in
        guard settings.authorizationStatus == .authorized || settings.authorizationStatus == .provisional else {return}
        let content=UNMutableNotificationContent()
        content.title="Some files need your attention"
        content.body="Open FilePilot Review inbox to summarise and choose what happens next for \(count) files"
        content.sound = .default
        center.add(UNNotificationRequest(identifier:"filepilot-review",content:content,trigger:nil)) { _ in }
    }
}
