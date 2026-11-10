import Foundation
import Darwin

let input = "University lecture assignment and course study notes"
let pointer = input.withCString { semanticRank($0) }!
let data = Data(String(cString:pointer).utf8)
free(pointer)
let result = try JSONSerialization.jsonObject(with:data) as! [String:Any]
if result["available"] as? Bool == true {
    let ranking=result["ranking"] as! [[String:Any]]
    precondition(ranking.count == 3)
    precondition(ranking.contains { $0["category"] as? String == "Education" })
    precondition(result["review_required"] as? Bool == true)
    precondition(result["scores_are_probabilities"] as? Bool == false)
    print("Synthetic local embedding smoke check passed")
} else {
    print("Local embedding unavailable fallback is explicit")
}
let invalid="not a valid bookmark".withCString { resolveRoot($0) }!
let invalidData=Data(String(cString:invalid).utf8)
free(invalid)
let rejection=try JSONSerialization.jsonObject(with:invalidData) as! [String:Any]
precondition(rejection["error"] != nil)
print("Invalid bookmark rejection passed")
