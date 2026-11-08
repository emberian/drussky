import Foundation
import LearningCore

extension RussianDomain {
    /// When a Russian memory counts as compiled, per modality: the floors are the response
    /// latency a learner shows once the answer no longer has to be worked out. They are set from
    /// what each activity measures (`ExerciseResult.responseMs`), not from an ideal speaker:
    ///   hear → pick / read → pick   from the clip's end (or the exercise shown) to the tap: a
    ///                               recognised word is picked in about a second; 2.5 s allows
    ///                               for reading three options.
    ///   English → build             tiles tapped into order after reading the meaning: several
    ///                               taps, so 6 s.
    ///   hear → write, English → write   from the first stroke to the last: the pen itself takes
    ///                               a few seconds for a word, and a hesitation mid-word shows as
    ///                               a longer one; 8 s.
    ///   see → say                   from the cue to the voice's onset: a word that is there comes
    ///                               within about a second; 1.5 s.
    ///   copying and tracing are never "compiled": there is nothing to retrieve.
    /// Changing a floor changes who counts as quick; the policy's other thresholds (three timed
    /// right answers, all of the last five right) are LearningCore's defaults.
    static let compilation = CompilationPolicy(floorsMs: [
        .hearPick: 2_500, .readPick: 2_500, .build: 6_000,
        .hearWrite: 8_000, .meaningWrite: 8_000, .say: 1_500,
        .copy: 0,  // never under the floor
    ])
}

extension KnowledgeTracker {
    var compilation: CompilationPolicy { RussianDomain.compilation }

    /// Whether `c` now comes without thinking in some modality.
    func isCompiled(_ c: ComponentID) -> Bool { !model.compiledModalities(of: c, policy: compilation).isEmpty }
}
