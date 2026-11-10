import Foundation
import PDFKit
import Darwin

func emit(_ value:[String:Any]) {
    if let data=try? JSONSerialization.data(withJSONObject:value) {
        FileHandle.standardOutput.write(data)
    }
}
var cpu=rlimit(rlim_cur:4,rlim_max:4)
guard setrlimit(RLIMIT_CPU,&cpu)==0 else {
    emit(["status":"skipped","reason":"CPU limit unavailable"])
    exit(1)
}
var payload=Data()
while payload.count<=2097152 {
    let chunk=FileHandle.standardInput.readData(ofLength:min(65536,2097153-payload.count))
    if chunk.isEmpty {break}
    payload.append(chunk)
}
guard payload.count<=2097152 && payload.starts(with:Data("%PDF-".utf8)) else {
    emit(["status":"skipped","reason":"PDF input size or signature rejected"])
    exit(0)
}
guard let document=PDFDocument(data:payload) else {
    emit(["status":"skipped","reason":"Invalid PDF"])
    exit(0)
}
guard !document.isEncrypted else {
    emit(["status":"skipped","reason":"Encrypted PDF"])
    exit(0)
}
var text=""
var inspected=0
var truncated=document.pageCount>5
for index in 0..<min(document.pageCount,5) {
    guard let page=document.page(at:index) else {continue}
    let value=page.string ?? ""
    let remaining=2000-text.count
    text+=String(value.prefix(remaining))
    inspected+=1
    if value.count>remaining {truncated=true;break}
    if text.count>=2000 {truncated = truncated || index+1<document.pageCount;break}
}
emit(["status":text.trimmingCharacters(in:.whitespacesAndNewlines).isEmpty ? "no_text":"ok","text":text,"pages_inspected":inspected,"truncated":truncated,"ocr":false])
