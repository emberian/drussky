import Foundation

/// Who made what the app bundles, from credits.json (apple/scripts/build_credits.py reads every
/// provenance manifest, the library, the fonts and the reference keys, and flags what nobody has
/// been credited for yet). A record, not a clearance: an entry whose `credit_status` is not
/// "ready" is shown with its gap, never hidden.
struct Credits: Decodable, Sendable {
    var version: Int
    var title: String
    var audit_date: String
    var model_authored_content: String
    var status: String
    /// Open items by kind: clip or entry ids the generator could not credit.
    var unresolved: [String: [String]]
    var authors: [Author]
    var groups: [Group]

    struct Author: Decodable, Sendable {
        var name: String
        var assets: [String]
    }

    struct Group: Decodable, Sendable {
        var kind: String
        var entries: [Entry]
    }

    struct Reference: Decodable, Sendable, Hashable {
        var lemma: String
        var source_url: String
        var history_url: String
    }

    /// One credited source. Which fields are present depends on the group; `attribution` is
    /// always the complete sentence the licence asks for.
    struct Entry: Decodable, Sendable, Identifiable {
        var id: String
        var title: String
        /// What a recording says (its index key); the title is the source file's name.
        var spoken: String?
        var author: String?
        var source_url: String?
        var source_urls: [String]?
        var translation_source_url: String?
        /// Normalised identifier (CC-BY-4.0, public-domain, unknown) and the licence as the source recorded it.
        var license: String?
        var recorded_license: String?
        var license_url: String?
        var license_text: String?
        var license_note: String?
        var attribution: String
        var credit_status: String
        var changes: String?
        /// Bundled clips (audio groups), font files, or repository paths (foundation_audio).
        var assets: [String]?
        var ru_contributors: [String]?
        var en_contributors: [String]?
        var audio_asset: String?
        var note: String?
        var requirement_note: String?
        var applies_when: String?
        var references: [Reference]?
        var model: String?
        var model_sha256: String?
        var source_revision: String?
        var revision: String?

        var isReady: Bool { credit_status == "ready" }
        /// The licence to show: as recorded, else the identifier; nil when none was recorded.
        var licenceName: String? { recorded_license ?? (license == "unknown" ? nil : license) }
    }

    func group(_ kind: String) -> [Entry] { groups.first { $0.kind == kind }?.entries ?? [] }

    /// Which credit entries own each bundled clip. Every clip in Audio/ must appear exactly once;
    /// foundation_audio names repository files, not clips, so it is left out.
    var clipOwners: [String: [String]] {
        var owners: [String: [String]] = [:]
        for kind in ["human_audio", "narration", "synthetic_audio"] {
            for entry in group(kind) {
                for asset in entry.assets ?? [] { owners[asset, default: []].append(entry.id) }
            }
        }
        return owners
    }

    static func load(from bundle: Bundle = .main) throws -> Credits {
        guard let url = bundle.url(forResource: "credits", withExtension: "json") else { throw CocoaError(.fileNoSuchFile) }
        return try JSONDecoder().decode(Credits.self, from: Data(contentsOf: url))
    }
}
