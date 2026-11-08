import Foundation

/// One measurement of something about a component: a speed, a share, a count. What the metric
/// means is the domain's business; the core only keeps the series and answers questions about it.
public struct MetricSample: Codable, Sendable, Hashable {
    public var t: Date
    public var value: Double
    /// A short domain note (a rate, a take's file), kept with the sample.
    public var note: String?

    public init(t: Date, value: Double, note: String? = nil) {
        self.t = t
        self.value = value
        self.note = note
    }
}

/// One line of the append-only metric log (`metrics.jsonl`): which component, which metric,
/// the sample. Fields are only ever added.
public struct MetricRecord: Codable, Sendable, Hashable {
    public var v: Int = 1
    public var component: ComponentID
    public var metric: String
    public var t: Date
    public var value: Double
    public var note: String?

    public init(component: ComponentID, metric: String, t: Date, value: Double, note: String? = nil) {
        self.component = component
        self.metric = metric
        self.t = t
        self.value = value
        self.note = note
    }

    public var sample: MetricSample { MetricSample(t: t, value: value, note: note) }
}

/// Time series of metrics per (component, metric): append, query, and compare one window with
/// the one before it. A pure fold over its records, like the knowledge model over its events.
public struct MetricStore: Sendable, Equatable {
    public struct Key: Hashable, Sendable, Codable {
        public var component: ComponentID
        public var metric: String
        public init(_ component: ComponentID, _ metric: String) {
            self.component = component
            self.metric = metric
        }
    }

    /// A window of samples against the window before it.
    public struct Change: Sendable, Equatable {
        /// Typical (median) value in the recent window and in the one before it.
        public var recent: Double
        public var earlier: Double
        public var recentCount: Int
        public var earlierCount: Int
        /// (recent − earlier) / earlier; nil when the earlier window is zero.
        public var relative: Double? { earlier == 0 ? nil : (recent - earlier) / earlier }
    }

    private var series: [Key: [MetricSample]] = [:]
    public private(set) var records: [MetricRecord] = []

    public init() {}

    public init(records: [MetricRecord]) {
        for r in records { apply(r) }
    }

    public mutating func apply(_ record: MetricRecord) {
        records.append(record)
        let key = Key(record.component, record.metric)
        var s = series[key] ?? []
        // Samples arrive in time order almost always; keep the series sorted regardless.
        if let last = s.last, last.t > record.t, let i = s.firstIndex(where: { $0.t > record.t }) {
            s.insert(record.sample, at: i)
        } else {
            s.append(record.sample)
        }
        series[key] = s
    }

    @discardableResult
    public mutating func append(_ value: Double, metric: String, component: ComponentID, at t: Date, note: String? = nil) -> MetricRecord {
        let r = MetricRecord(component: component, metric: metric, t: t, value: value, note: note)
        apply(r)
        return r
    }

    /// The samples of one series, oldest first.
    public func samples(_ metric: String, of component: ComponentID) -> [MetricSample] {
        series[Key(component, metric)] ?? []
    }

    public func latest(_ metric: String, of component: ComponentID) -> MetricSample? {
        samples(metric, of: component).last
    }

    /// Components that have at least one sample of `metric`, most recently measured first.
    public func components(with metric: String) -> [ComponentID] {
        series.filter { $0.key.metric == metric && !$0.value.isEmpty }
            .sorted { ($0.value.last?.t ?? .distantPast) > ($1.value.last?.t ?? .distantPast) }
            .map(\.key.component)
    }

    /// Every metric name seen for a component.
    public func metrics(of component: ComponentID) -> [String] {
        series.keys.filter { $0.component == component }.map(\.metric).sorted()
    }

    /// The series restricted to samples whose note is `note` (a rate, a condition).
    public func samples(_ metric: String, of component: ComponentID, note: String) -> [MetricSample] {
        samples(metric, of: component).filter { $0.note == note }
    }

    /// Samples within the `window` ending `at` compared with the samples in the window before it.
    /// Nil until each window has `minimum` samples: a change needs something on both sides.
    public func change(_ metric: String, of component: ComponentID, window: TimeInterval, at now: Date,
                       minimum: Int = 2, note: String? = nil) -> Change? {
        let all = note.map { samples(metric, of: component, note: $0) } ?? samples(metric, of: component)
        let recent = all.filter { $0.t > now.addingTimeInterval(-window) && $0.t <= now }.map(\.value)
        let earlier = all.filter { $0.t > now.addingTimeInterval(-2 * window) && $0.t <= now.addingTimeInterval(-window) }.map(\.value)
        guard recent.count >= minimum, earlier.count >= minimum,
              let r = Self.median(recent), let e = Self.median(earlier) else { return nil }
        return Change(recent: r, earlier: e, recentCount: recent.count, earlierCount: earlier.count)
    }

    /// The mean of the samples in the `window` ending `at` (nil when there are none).
    public func mean(_ metric: String, of component: ComponentID, window: TimeInterval, at now: Date, note: String? = nil) -> (value: Double, count: Int)? {
        let all = note.map { samples(metric, of: component, note: $0) } ?? samples(metric, of: component)
        let xs = all.filter { $0.t > now.addingTimeInterval(-window) && $0.t <= now }.map(\.value)
        guard !xs.isEmpty else { return nil }
        return (xs.reduce(0, +) / Double(xs.count), xs.count)
    }

    public static func median(_ xs: [Double]) -> Double? {
        guard !xs.isEmpty else { return nil }
        let s = xs.sorted()
        return s.count % 2 == 1 ? s[s.count / 2] : (s[s.count / 2 - 1] + s[s.count / 2]) / 2
    }
}

/// Metric records as JSON lines in one file, like `EventLog`: appends never rewrite earlier
/// lines, a line that fails to decode is skipped and kept.
public struct MetricLog: Sendable {
    public let url: URL
    public init(url: URL) { self.url = url }

    public func load() -> [MetricRecord] {
        guard let data = try? Data(contentsOf: url) else { return [] }
        let decoder = EventLog.decoder()
        return data.split(separator: UInt8(ascii: "\n")).compactMap { try? decoder.decode(MetricRecord.self, from: Data($0)) }
    }

    @discardableResult
    public func append(_ records: [MetricRecord]) -> Bool {
        guard !records.isEmpty else { return true }
        let encoder = EventLog.encoder()
        var blob = Data()
        for r in records {
            guard let line = try? encoder.encode(r) else { continue }
            blob.append(line)
            blob.append(UInt8(ascii: "\n"))
        }
        let fm = FileManager.default
        if !fm.fileExists(atPath: url.path) {
            try? fm.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            return fm.createFile(atPath: url.path, contents: blob)
        }
        guard let handle = try? FileHandle(forUpdating: url) else { return false }
        defer { try? handle.close() }
        do {
            let end = try handle.seekToEnd()
            if end > 0 {
                try handle.seek(toOffset: end - 1)
                if try handle.read(upToCount: 1) != Data([UInt8(ascii: "\n")]) { blob.insert(UInt8(ascii: "\n"), at: 0) }
                try handle.seekToEnd()
            }
            try handle.write(contentsOf: blob)
            return true
        } catch {
            return false
        }
    }
}
