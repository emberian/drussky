import Foundation

/// Frozen with its notebook exercise. References stay hidden until the first response is saved.
struct BaselineTask: Codable, Sendable, Hashable {
    enum Kind: String, Codable, Sendable { case reading, listening, speaking, writing }
    var id: String
    var kind: Kind
    var prompt: String
    var questions: [String]
    var reference: [String]
    var passage: String? = nil
    var audio: String? = nil
    var speaker: String? = nil
    /// A message played by its text (a listening.json weekly message, Model/Listening.swift) in
    /// the voice `speaker` names, when `audio` names no file.
    var text: String? = nil
}

/// The first attempt is immutable once sealed. Reflection is separate from that original.
struct BaselineResponse: Codable, Sendable, Hashable {
    var started: Date
    var text: String = ""
    var plays: Int = 0
    var helped: Bool = false
    var familiar: Bool = false
    var familiarVoice: Bool = false
    var sealed: Date? = nil
    var reflection: String? = nil
    /// Rough exposure in the preceding week, in the learner's own words.
    var exposure: String? = nil
}

extension BookPage {
    var isBaseline: Bool { exercises.contains { $0.baseline != nil } }
}

@MainActor
extension Book {
    /// Existing unfinished samples resume in place; no repeated material is labelled unseen.
    @discardableResult
    func openBaseline(listening forced: ListeningSet? = nil) -> Bool {
        if let index = pages.firstIndex(where: { $0.isBaseline && !$0.isFinished }) {
            goTo(index); return true
        }
        let used = Set(pages.flatMap { $0.exercises.compactMap { $0.baseline?.id } })
        guard let reading = BaselineBank.readings.first(where: { !used.contains($0.id) }) else { return false }
        // Five unheard messages: the reserved listening set at the highest support level the course
        // has reached (Model/Listening.swift `weekly_samples`), none of it used before; the bank of
        // human recordings when no reserved set is reached yet or all reached ones are spent.
        let weekly = forced ?? Listening.shared.weekly_samples.last { set in
            finished(coursePage: set.after_page) && !set.items.contains { used.contains($0.id) }
        }
        let messages = weekly.map { $0.items.map(BaselineBank.message) } ?? Array(BaselineBank.listening.filter { !used.contains($0.id) }.prefix(5))
        let tasks = [reading] + messages + [
            BaselineTask(id: "speak-\(reading.id)", kind: .speaking,
                         prompt: "You need to borrow a book. In Russian, ask where it is and explain when you can return it. Choose the details yourself; speak without a script.",
                         questions: [], reference: ["Could a listener tell which book you wanted, what you asked, and when you would return it? Note what they understood and any clarification they needed."]),
            BaselineTask(id: "write-\(reading.id)", kind: .writing,
                         prompt: "Write a short message in Russian: where you are, what happened today, and one plan for tomorrow. Keep this first paragraph as written.",
                         questions: [], reference: ["Which of your intended ideas came through? Keep any later correction on another page."])
        ]
        let exercises = tasks.map { task in
            var ex = Exercise(kind: .explain)
            ex.baseline = task
            return ex
        }
        insertWrittenPage(title: "What I could do this week", intro: "Try each task once before looking at the notes; your first words, ink and voice stay here.",
                          unitTitle: "Weekly sample", exercises: exercises, newWords: [], sourceID: "baseline:\(reading.id)")
        return true
    }

    /// Used by the capture view and tests. Further saves cannot overwrite a first response.
    @discardableResult
    func sealBaseline(_ ref: ExerciseRef, response: BaselineResponse) -> Bool {
        guard let page = page(id: ref.pageID), page.exercises.indices.contains(ref.index),
              page.exercises[ref.index].baseline != nil, page.results[ref.index].baseline?.sealed == nil else { return false }
        var saved = response
        saved.sealed = now()
        record(ref) { $0.baseline = saved; $0.status = .done; $0.attempts = 1 }
        return true
    }
}

/// Two parallel samples, deliberately outside the course and its practice/review pool.
/// These are provisional beginner texts, not a validated CEFR assessment.
enum BaselineBank {
    static let readings: [BaselineTask] = [
        BaselineTask(id: "station-book-a", kind: .reading,
            prompt: "Read this new scene. Answer in English or Russian, using the passage.",
            questions: ["What problem does Anna solve?", "Where is the book?", "When will Anna read?", "Why does Anna go to the kitchen?"],
            reference: ["She finds the book she wants to read.", "On the kitchen table.", "In the evening, after work.", "Pavel tells her the book is there; she goes to get it."],
            passage: "А́нна живёт на ста́нции. У́тром она́ хо́чет чита́ть, но кни́ги нет в ко́мнате. Она́ идёт к Па́влу. «Где моя́ кни́га?» — спра́шивает она́. Па́вел говори́т: «Кни́га на столе́, на ку́хне. Я чита́л её вчера́». А́нна идёт на ку́хню. Там она́ ви́дит кни́гу и письмо́. Письмо́ от бра́та. Он пи́шет, что за́втра бу́дет на ста́нции. А́нна берёт кни́гу, но сейча́с уже́ пора́ рабо́тать. Она́ бу́дет чита́ть ве́чером, по́сле рабо́ты."),
        BaselineTask(id: "station-letter-b", kind: .reading,
            prompt: "Read this new scene. Answer in English or Russian, using the passage.",
            questions: ["What problem does Pavel solve?", "Where is the letter?", "When will Pavel write?", "Why does Pavel go to the kitchen?"],
            reference: ["He finds the letter he wants to read.", "On the kitchen table, under a book.", "In the morning, before work.", "Anna tells him the letter is there; he goes to get it."],
            passage: "Па́вел живёт на ста́нции. Ве́чером он хо́чет чита́ть письмо́, но письма́ нет в ко́мнате. Он идёт к А́нне. «Где моё письмо́?» — спра́шивает он. А́нна говори́т: «Письмо́ на столе́, на ку́хне, под кни́гой». Па́вел идёт на ку́хню. Там он ви́дит кни́гу и письмо́. Письмо́ от сестры́. Она́ пи́шет, что за́втра бу́дет на ста́нции. Па́вел чита́ет письмо́. Он хо́чет писа́ть сестре́, но сейча́с уже́ пора́ спать. Он бу́дет писа́ть у́тром, до рабо́ты.")
    ]

    static let listening: [BaselineTask] = [
        message("Она́ не хо́чет говори́ть об э́том.", "ona-ne-khochet-govorit-ob-etom-4a7c73.m4a", "Inego", "Who is mentioned, and what do they not want to do?", "She does not want to talk about it."),
        message("Ду́маю, что он сейча́с до́ма.", "dumayu-chto-on-seychas-doma-7d08fe.m4a", "EugeneGS", "Where is the person, and is the speaker certain?", "The speaker thinks he is at home now; it is a belief, not certainty."),
        message("У тебя́ сейча́с есть вре́мя?", "u-tebya-seychas-est-vremya-f46ee4.m4a", "fjay69", "What is being asked, and about when?", "Whether you have time / are free now."),
        message("Вы не хоти́те здесь рабо́тать?", "vy-ne-khotite-zdes-rabotat-863da5.m4a", "retr0ra1n", "What activity and place are being asked about?", "Whether you don't want to work here."),
        message("Э́то не наш по́езд.", "eto-ne-nash-poezd-d662cf.m4a", "Alex_M", "What is identified, and what is the speaker saying about it?", "It is a train, but it is not ours."),
        message("Ты зна́ешь, кто она́ така́я?", "ty-znaesh-kto-ona-takaya-dab71b.m4a", "Inego", "What does the speaker want to find out?", "Whether you know who she is."),
        message("Я ещё не до́ма.", "ya-eshchyo-ne-doma-a2ae6f.m4a", "EugeneGS", "Where has the speaker not arrived, and has that changed yet?", "The speaker is not home yet."),
        message("Я не понима́ю э́того сло́ва.", "ya-ne-ponimayu-etogo-slova-d6e20b.m4a", "fjay69", "What problem is the speaker reporting?", "They do not understand this word."),
        message("Они́ не в го́роде.", "oni-ne-v-gorode-4fb7b1.m4a", "retr0ra1n", "Who is mentioned, and where are they not?", "They are not in the city."),
        message("Иди́ сейча́с домо́й.", "idi-seychas-domoy-4b4519.m4a", "Alex_M", "What should the listener do, and when?", "Go home now.")
    ]

    private static func message(_ ru: String, _ audio: String, _ speaker: String, _ question: String, _ answer: String) -> BaselineTask {
        BaselineTask(id: audio, kind: .listening, prompt: "Listen before reading. Write what you understood.", questions: [question], reference: [ru, answer], audio: audio, speaker: speaker)
    }

    /// A reserved weekly message as a sample: its questions answered in English on the lines, the
    /// Russian and the model meanings shown only once the first response is sealed. Never graded.
    static func message(_ item: ListeningItem) -> BaselineTask {
        let questions = item.questions ?? []
        let reference = [item.ru ?? "", item.en] + questions.map { q in "\(q.prompt) — \(q.answer_en ?? "")" }
        return BaselineTask(id: item.id, kind: .listening, prompt: "Listen before reading. Answer in English, in your own words.",
                            questions: questions.map(\.prompt), reference: reference.filter { !$0.isEmpty },
                            speaker: item.voice(), text: item.ru)
    }
}
