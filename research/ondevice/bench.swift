import Foundation
import FoundationModels

// On-device half of the tutor benchmark: runs cases.json through the exact prompts and schemas
// the app uses (apple/Drussky/OnDevice/OnDeviceSchemas.swift, compiled alongside) and writes
// ondevice.json. Build and run: see research/ONDEVICE_TUTOR.md.
//   bench <cases.json> <out.json> [gloss,ending,diary,lemma]

struct Cases: Decodable {
    struct Gloss: Decodable { var id, word, context, reference: String }
    struct Ending: Decodable { var id, sentence, word, key: String }
    struct Diary: Decodable { var id, prompt, text, gold: String }
    struct Lemma: Decodable { var id, form: String; var gold: [String] }
    var glosses: [Gloss]; var endings: [Ending]; var diaries: [Diary]; var lemmas: [Lemma]
}

struct Result: Encodable {
    var id: String
    var seconds: Double
    var answer: String?        // gloss / quick answer text
    var lemma: String?
    var json: String?          // diary / lemma table as JSON
    var error: String?
}

func encode<T: Encodable>(_ v: T) -> String {
    let e = JSONEncoder(); e.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
    return String(decoding: (try? e.encode(v)) ?? Data(), as: UTF8.self)
}

struct DiaryOut: Encodable {
    struct Edit: Encodable { var span, replacement, category, why, en: String }
    struct Teach: Encodable { var title, note: String }
    var corrected: String; var edits: [Edit]; var praise: String; var teach: [Teach]
    init(_ c: OnDeviceDiaryCorrection) {
        corrected = c.corrected
        edits = c.edits.map { .init(span: $0.span, replacement: $0.replacement, category: $0.category.rawValue, why: $0.why, en: $0.en) }
        praise = c.praise
        teach = c.teach.map { .init(title: $0.title, note: $0.note) }
    }
}

struct LemmaOut: Encodable { var form, lemma, gloss: String }

@main
struct Bench {
    static func main() async throws {
        let args = CommandLine.arguments
        guard args.count >= 3 else { print("usage: bench cases.json out.json [tasks]"); return }
        let cases = try JSONDecoder().decode(Cases.self, from: Data(contentsOf: URL(fileURLWithPath: args[1])))
        let tasks = Set((args.count > 3 ? args[3] : "gloss,ending,diary,lemma").split(separator: ",").map(String.init))
        var out: [String: [Result]] = [:]
        print("availability:", OnDeviceModel.availability)
        guard OnDeviceModel.isAvailable else { return }

        func timed(_ id: String, _ body: () async throws -> Result) async -> Result {
            let t0 = Date()
            do { var r = try await body(); r.seconds = Date().timeIntervalSince(t0); return r }
            catch { return Result(id: id, seconds: Date().timeIntervalSince(t0), error: "\(error)") }
        }

        if tasks.contains("gloss") {
            var rs: [Result] = []
            for c in cases.glosses {
                let r = await timed(c.id) {
                    let g = try await OnDeviceModel.gloss(word: c.word, context: c.context)
                    return Result(id: c.id, seconds: 0, answer: g.gloss, lemma: g.lemma)
                }
                print(c.id, c.word, "→", r.answer ?? r.error ?? "", "|", r.lemma ?? "", String(format: "%.1fs", r.seconds))
                rs.append(r)
            }
            out["glosses"] = rs
        }
        if tasks.contains("ending") {
            var rs: [Result] = []
            for c in cases.endings {
                let r = await timed(c.id) {
                    let a = try await OnDeviceModel.quickAnswer(question: "Why does «\(c.word)» have this ending here?",
                                                                context: "The sentence on the page: \(c.sentence)")
                    return Result(id: c.id, seconds: 0, answer: a)
                }
                print(c.id, c.word, "→", (r.answer ?? r.error ?? "").prefix(160), String(format: "%.1fs", r.seconds))
                rs.append(r)
            }
            out["endings"] = rs
        }
        if tasks.contains("diary") {
            var rs: [Result] = []
            let briefing = "Letters met: all of them. Words they know: мама, папа, брат, книга, читать, школа, дом, друг, кофе, чай, парк."
            for c in cases.diaries {
                let r = await timed(c.id) {
                    let d = try await OnDeviceModel.correctDiary(text: c.text, prompt: c.prompt, briefing: briefing)
                    return Result(id: c.id, seconds: 0, json: encode(DiaryOut(d)))
                }
                print(c.id, "→", (r.json ?? r.error ?? "").prefix(300), String(format: "%.1fs", r.seconds))
                rs.append(r)
            }
            out["diaries"] = rs
        }
        if tasks.contains("lemma") {
            var rs: [Result] = []
            let batch = 10
            for start in stride(from: 0, to: cases.lemmas.count, by: batch) {
                let chunk = Array(cases.lemmas[start..<min(start + batch, cases.lemmas.count)])
                let t0 = Date()
                do {
                    let table = try await OnDeviceModel.lemmatize(chunk.map(\.form))
                    let secs = Date().timeIntervalSince(t0) / Double(chunk.count)
                    for c in chunk {
                        let row = table.rows.first { $0.form == c.form }
                        rs.append(Result(id: c.id, seconds: secs, answer: row?.gloss, lemma: row?.lemma,
                                         error: row == nil ? "form missing from table" : nil))
                        print(c.id, c.form, "→", row?.lemma ?? "(missing)", "|", row?.gloss ?? "", "gold", c.gold)
                    }
                } catch {
                    for c in chunk { rs.append(Result(id: c.id, seconds: 0, error: "\(error)")) }
                    print("batch at \(start) failed: \(error)")
                }
            }
            out["lemmas"] = rs
        }
        try Data(encode(out).utf8).write(to: URL(fileURLWithPath: args[2]))
        print("wrote", args[2])
    }
}
