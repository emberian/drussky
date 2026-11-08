// Evaluation harness for the pronunciation engine on the bundled recordings (macOS CLI; see
// run.sh and Drussky/Pronunciation/README.md).
//   peval pairs                         minimal pairs: each word's audio against both targets
//   peval native [kind] [limit] [-v]    false alarms on native takes (kind: human_recording, ai_generated_speech)
//   peval one target[:audioword] ...    one report per spec
// Env: DRUSSKY_APPLE (apple/ dir), DRUSSKY_AUDIO (audio dir, default <apple>/Drussky/Resources/Audio).
import Foundation

let env = ProcessInfo.processInfo.environment
let worktree = env["DRUSSKY_APPLE"] ?? FileManager.default.currentDirectoryPath
let audioDir = URL(fileURLWithPath: env["DRUSSKY_AUDIO"] ?? worktree + "/Drussky/Resources/Audio")
let modelsDir = URL(fileURLWithPath: worktree + "/Drussky/Resources/Models")
let args = CommandLine.arguments
let mode = args.count > 1 ? args[1] : "pairs"

func json(_ url: URL) -> Any { try! JSONSerialization.jsonObject(with: Data(contentsOf: url)) }
var index: [String: String] = [:]
var kinds: [String: String] = [:]
for f in ["index.json", "index-pairs.json"] { for (k, v) in json(audioDir.appendingPathComponent(f)) as! [String: String] { index[k] = v } }
for f in ["sources.json", "sources-pairs.json"] {
    for (k, v) in json(audioDir.appendingPathComponent(f)) as! [String: [String: Any]] { kinds[k] = v["kind"] as? String ?? "?" }
}
// Stress-marked spellings from the course, by audio key.
var stressed: [String: String] = [:]
@MainActor func walk(_ x: Any) {
    if let s = x as? String, s.unicodeScalars.contains("\u{301}") {
        let k = RussianText.key(s)
        if stressed[k] == nil || s.count < stressed[k]!.count { stressed[k] = s.trimmingCharacters(in: .whitespaces) }
    } else if let d = x as? [String: Any] { d.values.forEach(walk) } else if let a = x as? [Any] { a.forEach(walk) }
}
walk(json(URL(fileURLWithPath: worktree + "/Drussky/Resources/course.json")))
walk(json(URL(fileURLWithPath: worktree + "/Drussky/Resources/library.json")))

let recognizer = try! PhoneRecognizer(directory: modelsDir)
let analyzer = PhoneticAnalyzer(recognizer: recognizer)

@MainActor func audio(for word: String) -> URL? {
    let f = index[word] ?? index[RussianText.unstressed(word)] ?? index[RussianText.key(word)]
    return f.map { audioDir.appendingPathComponent($0) }.flatMap { FileManager.default.fileExists(atPath: $0.path) ? $0 : nil }
}

func describe(_ r: PronunciationReport) -> String {
    let s = r.sounds.map { j -> String in
        let e = j.expected?.ipa ?? "∅", h = j.heard?.ipa ?? "∅"
        switch j.verdict {
        case .good: return e
        case .close: return e + "~"
        case .substituted: return "[\(e)→\(h)]"
        case .missing: return "[\(e)→∅]"
        case .extra: return "[+\(h)]"
        }
    }.joined(separator: " ")
    let syl = r.syllables.map { ($0.isStressed ? "ˈ" : "") + $0.text + ($0.prominence.map { String(format: "(%.2f)", $0) } ?? "") }.joined(separator: "-")
    return "\(s)  | \(syl) stress→\(r.stressedSyllable.map(String.init) ?? "-") acc \(String(format: "%.2f", r.accuracy))"
}

struct Flag { var kind: Mistake.Kind?; var verdict: PhoneJudgement.Verdict; var expected: String; var index: Int = -1 }

func kindsOf(_ r: PronunciationReport, target: String) -> [Flag] {
    // Re-derive the mistake kinds by matching heard ipa against the options for each sound.
    let t = RussianG2P.transcribe(target)
    var out: [Flag] = []
    var e = 0
    for j in r.sounds {
        guard j.expected != nil else { out.append(Flag(kind: nil, verdict: .extra, expected: "")); continue }
        defer { e += 1 }
        let s = t.sounds[e]
        if j.verdict == .substituted, let h = j.heard?.ipa {
            out.append(Flag(kind: PhoneSet.options(for: s).errors.first { $0.ipa == h }?.kind, verdict: .substituted, expected: s.ipa, index: e))
        } else if j.verdict == .missing {
            out.append(Flag(kind: nil, verdict: .missing, expected: s.ipa))
        }
    }
    return out
}

func contrastKinds(_ c: String) -> Set<Mistake.Kind> {
    switch c {
    case "y-i": [.yi]
    case "hard-soft": [.hardened, .softened]
    case "sh-shch": [.sh, .shch]
    case "zh-sh": [.devoicing, .voicing]
    case "reduction", "stress": [.reduction, .fullVowel]
    default: []
    }
}

do {
    if mode == "pairs" {
        let pairs = (json(URL(fileURLWithPath: worktree + "/Drussky/Resources/pairs.json")) as! [String: Any])["pairs"] as! [[String: Any]]
        var stats: [String: (tp: Int, fn: Int, fp: Int, same: Int, stressHit: Int, stressFA: Int)] = [:]
        for p in pairs {
            let c = p["contrast"] as! String
            let words = (p["words"] as! [[String: String]]).map { $0["ru"]! }
            let homophone = ["луг", "код", "плод"].contains(words[0])
            let key = homophone ? "zh-sh (final devoicing homophones)" : c
            for (a, b) in [(words[0], words[1]), (words[1], words[0])] {
                guard let ua = audio(for: a), let ub = audio(for: b) else { print("no audio", a, b); continue }
                for (au, word, cross) in [(ua, a, false), (ub, b, true)] {
                    let r = try! await analyzer.analyze(take: au, target: a, model: nil)
                    let flags = kindsOf(r, target: a)
                    // Where the two words differ, in the target's sounds.
                    let ta = RussianG2P.transcribe(a), tb = RussianG2P.transcribe(b)
                    var diff = Set<Int>()
                    for st in Align.run(ta.sounds.map(\.ipa), tb.sounds.map(\.ipa)) {
                        if case .substitute(let i, _) = st { diff.insert(i) }
                        if case .match(let i, let j) = st, ta.sounds[i].ipa != tb.sounds[j].ipa { diff.insert(i) }
                        if case .delete(let i) = st { diff.insert(i) }
                    }
                    for (i, s) in ta.sounds.enumerated() where s.isVowel && (c == "stress" || c == "reduction") { diff.insert(i) }
                    let hit = flags.contains { f in (f.kind.map(contrastKinds(c).contains) ?? false) && (!cross || diff.contains(f.index)) }
                    let st = (r.tips.contains { $0.contrast == "stress" })
                    var s = stats[key] ?? (0, 0, 0, 0, 0, 0)
                    let flagged = hit || ((c == "stress" || c == "reduction") && st)
                    if cross && !homophone { if flagged { s.tp += 1 } else { s.fn += 1 }; if st { s.stressHit += 1 } }
                    else { s.same += 1; if flagged { s.fp += 1 }; if st { s.stressFA += 1 } }
                    stats[key] = s
                    print(String(format: "%-10@ %-8@ target %@ audio %@ [%@]  %@", c as NSString, (cross ? "CROSS" : "same") as NSString,
                                 a as NSString, word as NSString, (kinds[au.lastPathComponent] ?? "?").prefix(5) as CVarArg, describe(r)))
                    for t in r.tips { print("            tip(\(t.contrast ?? "-")): \(t.message)") }
                }
            }
        }
        print("\ncontrast                 TP FN FP same  recall precision falseAlarm  stressTip(cross/same)")
        for (k, s) in stats.sorted(by: { $0.key < $1.key }) {
            let rec = Double(s.tp) / Double(max(1, s.tp + s.fn)), prec = Double(s.tp) / Double(max(1, s.tp + s.fp))
            print(String(format: "%-24@ %2d %2d %2d %4d   %.2f   %.2f      %.2f      %d/%d", k as NSString, s.tp, s.fn, s.fp, s.same, rec, prec,
                         Double(s.fp) / Double(max(1, s.same)), s.stressHit, s.stressFA))
        }
    } else if mode == "native" {
        let kindFilter = args.count > 2 ? args[2] : "human_recording"
        let limit = args.count > 3 ? Int(args[3])! : 100000
        var n = 0, sounds = 0, subs = 0, miss = 0, close = 0, extras = 0, clipsWithTip = 0
        var perKind: [String: Int] = [:]
        var times: [Double] = []
        var exposure: [Mistake.Kind: Int] = [:]
        for (key, file) in index.sorted(by: { $0.key < $1.key }) {
            guard kinds[file] == kindFilter, n < limit else { continue }
            let url = audioDir.appendingPathComponent(file)
            guard FileManager.default.fileExists(atPath: url.path) else { continue }
            let target = stressed[RussianText.key(key)] ?? key
            let t0 = Date()
            guard let r = try? await analyzer.analyze(take: url, target: target, model: nil) else { continue }
            times.append(Date().timeIntervalSince(t0))
            n += 1
            let t = RussianG2P.transcribe(target)
            for s in t.sounds { for m in PhoneSet.options(for: s).errors { exposure[m.kind, default: 0] += 1 } }
            for j in r.sounds {
                switch j.verdict {
                case .good: sounds += 1
                case .close: sounds += 1; close += 1
                case .substituted: sounds += 1; subs += 1
                case .missing: sounds += 1; miss += 1
                case .extra: extras += 1
                }
            }
            for f in kindsOf(r, target: target) where f.verdict == .substituted { perKind[f.kind?.rawValue ?? "other", default: 0] += 1 }
            if !r.tips.isEmpty { clipsWithTip += 1 }
            if args.contains("-v") || r.accuracy < 0.8 { print(target, "→", describe(r)); for tp in r.tips { print("     tip(\(tp.contrast ?? "-")): \(tp.message)") } }
        }
        print("\n\(kindFilter): \(n) clips, \(sounds) expected sounds")
        print(String(format: "substituted %.1f%%  missing %.1f%%  close %.1f%%  extras %d  clips with a tip %.0f%%", Double(subs) / Double(sounds) * 100,
                     Double(miss) / Double(sounds) * 100, Double(close) / Double(sounds) * 100, extras, Double(clipsWithTip) / Double(max(1, n)) * 100))
        print("false alarms by kind (flags / sounds where that mistake is possible):")
        for (k, v) in perKind.sorted(by: { $0.value > $1.value }) {
            let ex = Mistake.Kind(rawValue: k).flatMap { exposure[$0] } ?? 0
            print(String(format: "  %-10@ %4d / %4d = %.1f%%", k as NSString, v, ex, Double(v) / Double(max(1, ex)) * 100))
        }
        let st = times.sorted()
        if !st.isEmpty { print(String(format: "analysis time median %.0f ms, p90 %.0f ms", st[st.count / 2] * 1000, st[st.count * 9 / 10] * 1000)) }
    } else {
        // one: eval one target:audioword target2 ...  (audio word defaults to the target)
        for spec in args.dropFirst(2) {
            let parts = spec.split(separator: ":").map(String.init)
            let target = parts[0], aw = parts.count > 1 ? parts[1] : parts[0]
            print("== \(target) (audio \(aw))")
            let r = try! await analyzer.analyze(take: audio(for: aw)!, target: target, model: nil)
            print(describe(r))
            print("heard:", r.heard.map(\.ipa).joined(separator: " "))
            for t in r.tips { print("tip(\(t.contrast ?? "-")): \(t.message)") }
        }
    }
}
