import Foundation

/// A rule-based Russian lemmatizer for words outside the course: a closed table of function
/// words and irregular forms, then suffix rules that strip an inflection and restore a
/// dictionary ending, preferring a result the vocabulary knows. Homonymous endings (-а, -е,
/// -и, -у) make it wrong often, so goal analysis treats it as a stop-gap until the tutor's
/// lemma table arrives; its value is that the forms of one word usually collapse together.
struct RussianLemmatizer: Sendable {
    /// Dictionary forms the lemmatizer may trust: the course lexicon, lemmas.json, tutor lemmas.
    var vocabulary: Set<String>

    init(vocabulary: Set<String> = []) {
        self.vocabulary = Set(vocabulary.map { $0.replacingOccurrences(of: "ё", with: "е") }).union(Self.closedClass.values)
    }

    /// Function words, pronouns and the most irregular verbs and nouns. Keys without ё (RussianText.key form).
    static let closedClass: [String: String] = {
        var t: [String: String] = [:]
        func add(_ lemma: String, _ forms: String) { for f in forms.split(separator: " ") { t[String(f)] = lemma } }
        add("я", "я меня мне мной мною")
        add("ты", "ты тебя тебе тобой тобою")
        add("он", "он его него ему нему им ним нем")
        add("она", "она ее нее ей ней ею нею")
        add("оно", "оно")
        add("мы", "мы нас нам нами")
        add("вы", "вы вас вам вами")
        add("они", "они их них им ним ими ними")
        add("себя", "себя себе собой собою")
        add("кто", "кто кого кому кем ком")
        add("что", "что чего чему чем")
        add("этот", "этот эта это эти этого этой этому этим этих этими эту")
        add("тот", "тот та те того той тому тем тех теми ту")
        add("весь", "весь вся все всего всей всему всем всех всеми всю")
        add("сам", "сам сама само сами самого самой самому самим самих самими саму")
        add("мой", "мой моя мое мои моего моей моему моим моих моими мою")
        add("твой", "твой твоя твое твои твоего твоей твоему твоим твоих твоими твою")
        add("свой", "свой своя свое свои своего своей своему своим своих своими свою")
        add("наш", "наш наша наше наши нашего нашей нашему нашим наших нашими нашу")
        add("ваш", "ваш ваша ваше ваши вашего вашей вашему вашим ваших вашими вашу")
        add("который", "который которая которое которые которого которой которому которым которых которыми которую")
        add("какой", "какой какая какое какие какого какому каким каких какими какую")
        add("такой", "такой такая такое такие такого такому таким таких такими такую")
        add("один", "один одна одно одни одного одной одному одним одних одними одну")
        add("два", "два две двух двум двумя")
        add("три", "три трех трем тремя")
        add("четыре", "четыре четырех четырем четырьмя")
        add("никто", "никто никого никому никем")
        add("ничто", "ничто ничего ничему ничем")
        add("быть", "быть есть был была было были буду будешь будет будем будете будут будь будьте")
        add("идти", "идти иду идешь идет идем идете идут шел шла шло шли иди идите")
        add("пойти", "пойти пойду пойдешь пойдет пойдем пойдете пойдут пошел пошла пошло пошли")
        add("прийти", "прийти приду придешь придет придем придете придут пришел пришла пришло пришли")
        add("уйти", "уйти уйду уйдет уйдут ушел ушла ушло ушли")
        add("выйти", "выйти выйду выйдет выйдут вышел вышла вышло вышли")
        add("найти", "найти найду найдешь найдет найдем найдут нашел нашла нашло нашли")
        add("мочь", "мочь могу можешь может можем можете могут мог могла могло могли")
        add("хотеть", "хотеть хочу хочешь хочет хотим хотите хотят хотел хотела хотело хотели")
        add("есть", "ем ешь ест едим едите едят ел ела ело ели")
        add("дать", "дать дам дашь даст дадим дадите дадут дал дала дало дали")
        add("давать", "давать даю даешь дает даем даете дают давал давала давало давали")
        add("взять", "взять возьму возьмешь возьмет возьмем возьмете возьмут взял взяла взяло взяли")
        add("сказать", "сказать скажу скажешь скажет скажем скажете скажут сказал сказала сказало сказали скажи")
        add("писать", "писать пишу пишешь пишет пишем пишете пишут писал писала писало писали")
        add("жить", "жить живу живешь живет живем живете живут жил жила жило жили")
        add("ехать", "ехать еду едешь едет едем едете едут ехал ехала ехало ехали")
        add("стать", "стать стану станешь станет станем станете станут стал стала стало стали")
        add("начать", "начать начну начнешь начнет начнем начнете начнут начал начала начало начали")
        add("понять", "понять пойму поймешь поймет поймем поймете поймут понял поняла поняло поняли")
        add("брать", "брать беру берешь берет берем берете берут брал брала брало брали")
        add("ждать", "ждать жду ждешь ждет ждем ждете ждут ждал ждала ждало ждали")
        add("звать", "звать зову зовешь зовет зовем зовете зовут звал звала звало звали")
        add("нести", "нести несу несешь несет несем несете несут нес несла несло несли")
        add("вести", "вести веду ведешь ведет ведем ведете ведут вел вела вело вели")
        add("везти", "везти везу везет везут вез везла везло везли")
        add("расти", "расти расту растет растут рос росла росло росли")
        add("умереть", "умереть умру умрет умрут умер умерла умерло умерли")
        add("сесть", "сесть сяду сядет сядут сел села село сели")
        add("лечь", "лечь лягу ляжет лягут лег легла легло легли")
        add("спать", "спать сплю спишь спит спим спите спят спал спала спало спали")
        add("пить", "пить пью пьешь пьет пьем пьете пьют пил пила пило пили")
        add("петь", "петь пою поешь поет поем поете поют пел пела пело пели")
        add("бить", "бить бью бьешь бьет бьем бьете бьют бил била било били")
        add("видеть", "видеть вижу видишь видит видим видите видят видел видела видело видели")
        add("смотреть", "смотреть смотрю смотришь смотрит смотрим смотрите смотрят смотрел смотрела смотрело смотрели")
        add("сидеть", "сидеть сижу сидишь сидит сидим сидите сидят сидел сидела сидело сидели")
        add("лежать", "лежать лежу лежишь лежит лежим лежите лежат лежал лежала лежало лежали")
        add("стоять", "стоять стою стоишь стоит стоим стоите стоят стоял стояла стояло стояли")
        add("держать", "держать держу держишь держит держим держите держат держал держала держало держали")
        add("слышать", "слышать слышу слышишь слышит слышим слышите слышат слышал слышала слышало слышали")
        add("молчать", "молчать молчу молчишь молчит молчим молчите молчат молчал молчала молчало молчали")
        add("кричать", "кричать кричу кричишь кричит кричим кричите кричат кричал кричала кричало кричали")
        add("бежать", "бежать бегу бежишь бежит бежим бежите бегут бежал бежала бежало бежали")
        add("плакать", "плакать плачу плачешь плачет плачем плачете плачут плакал плакала плакало плакали")
        add("искать", "искать ищу ищешь ищет ищем ищете ищут искал искала искало искали")
        add("класть", "класть кладу кладет кладут клал клала клали")
        add("казаться", "казаться кажусь кажется кажутся казался казалась казалось казались")
        add("бояться", "бояться боюсь боишься боится боимся боитесь боятся боялся боялась боялось боялись")
        add("смеяться", "смеяться смеюсь смеешься смеется смеемся смеетесь смеются смеялся смеялась смеялось смеялись")
        add("ребенок", "ребенок ребенка ребенку ребенком ребенке дети детей детям детьми детях")
        add("человек", "человек человека человеку человеком человеке люди людей людям людьми людях")
        add("мать", "мать матери матерью матерей матерям матерями")
        add("дочь", "дочь дочери дочерью дочерей дочерям")
        add("время", "время времени временем времена времен временам временами")
        add("имя", "имя имени именем имена имен именам")
        add("год", "год года году годом годе годы лет годам годами годах")
        add("день", "день дня дню днем дне дни дней дням днями днях")
        add("глаз", "глаз глаза глазу глазом глазе глазам глазами глазах")
        add("рука", "рука руки руке руку рукой рукою рук рукам руками руках")
        add("нога", "нога ноги ноге ногу ногой ног ногам ногами ногах")
        add("голова", "голова головы голове голову головой голов головам головами головах")
        add("слово", "слово слова слову словом слове слов словам словами словах")
        add("дело", "дело дела делу делом деле дел делам делами делах")
        add("окно", "окно окна окну окном окне окон окнам окнами окнах")
        add("дом", "дом дома дому домом доме домов домам домами домах")
        add("друг", "друг друга другу другом друге друзья друзей друзьям друзьями друзьях")
        add("сын", "сын сына сыну сыном сыне сыновья сыновей сыновьям сыновьями")
        add("брат", "брат брата брату братом брате братья братьев братьям братьями")
        add("отец", "отец отца отцу отцом отце отцы отцов отцам отцами отцах")
        add("мышь", "мышь мыши мышью мышей мышам мышами мышах")
        add("лев", "лев льва льву львом льве львы львов львам львами львах")
        add("вода", "вода воды воде воду водой водою вод водам водами водах")
        add("земля", "земля земли земле землю землей землею земель землям землями землях")
        add("сестра", "сестра сестры сестре сестру сестрой сестер сестрам сестрами сестрах")
        add("жена", "жена жены жене жену женой жен женам женами женах")
        add("ночь", "ночь ночи ночью ночей ночам ночами ночах")
        add("путь", "путь пути путем путей путям путями путях")
        add("мир", "мир мира миру миром мире миры миров мирам мирами")
        add("час", "час часа часу часом часе часы часов часам часами часах")
        add("раз", "раз раза разу разом разе разы")
        add("конец", "конец конца концу концом конце концы концов")
        add("город", "город города городу городом городе города городов городам городами городах")
        for w in ("еще уже более больше лучше меньше хорошо нет не ни да же ли бы то тоже также или если чтобы потому почему "
                  + "поэтому когда тогда где куда здесь там тут сюда туда теперь потом сейчас сегодня вчера завтра всегда никогда "
                  + "очень только просто даже почти опять снова вдруг после перед через между около мимо назад вперед вместе "
                  + "можно нужно надо нельзя много немного мало сколько несколько как так а и но в во на с со к ко у о об обо от "
                  + "ото до из изо за по под подо над при для без безо про вот вон ну ведь хотя пока лишь либо нибудь кое "
                  + "вместо кроме среди сквозь вокруг против вдоль ради будто словно точно").split(separator: " ").map(String.init)
                  + Array(RussianText.functionWords.union(RussianText.clitics)) {
            t[w] = w
        }
        return t
    }()

    /// Masculine adjectives in stressed -ой (otherwise -ой reads as a feminine instrumental).
    static let ojAdjectives: Set<String> = ["большой", "другой", "родной", "живой", "плохой", "молодой", "дорогой", "простой",
                                             "пустой", "прямой", "святой", "слепой", "глухой", "худой", "сухой", "золотой", "земной",
                                             "иной", "чужой", "немой", "седой", "босой", "нагой", "крутой", "густой", "смешной",
                                             "больной", "лесной", "ночной", "дневной", "морской", "речной", "городской", "людской",
                                             "мировой", "цветной", "второй", "шестой", "седьмой", "восьмой", "любой", "сырой", "тупой",
                                             "скупой", "косой", "хромой", "кривой", "дурной", "стальной", "ручной", "ножной", "глазной",
                                             "зубной", "грудной", "срамной", "сквозной", "столовой", "вселенской"]

    /// Adjectives in soft -ний, so that -ние is read as their plural and not as a noun in -ние.
    static let softAdjectives: Set<String> = ["последний", "ранний", "средний", "летний", "зимний", "весенний", "осенний",
                                               "древний", "дальний", "ближний", "нижний", "верхний", "крайний", "прежний",
                                               "внешний", "внутренний", "вечерний", "утренний", "домашний", "сегодняшний",
                                               "вчерашний", "здешний", "лишний", "искренний", "поздний", "соседний", "передний",
                                               "задний", "синий", "давний", "нынешний", "тогдашний", "вешний", "сторонний",
                                               "посторонний", "субботний", "ранний", "поздний", "третий"]

    private static let vowels: Set<Character> = ["а", "е", "ё", "и", "о", "у", "ы", "э", "ю", "я"]
    private static let velarsAndHushing: Set<Character> = ["к", "г", "х", "ж", "ш", "ч", "щ"]
    private static let velars: Set<Character> = ["к", "г", "х"]
    private static let hushing: Set<Character> = ["ж", "ш", "ч", "щ"]
    private static let nounsInIt: Set<String> = ["магнит", "визит", "аппетит", "гранит", "кредит", "бандит", "сателлит", "спирит"]

    /// The dictionary form of a token (already a RussianText.key: lowercase, no stress).
    func lemma(_ token: String) -> String {
        let t = token.replacingOccurrences(of: "ё", with: "е")
        if let c = Self.closedClass[t] { return c }
        guard t.count > 2, t.allSatisfy({ ("а"..."я").contains($0) }) else { return t }
        if vocabulary.contains(t) { return t }
        if t.hasSuffix("ся") || t.hasSuffix("сь"), t.count > 4 {
            let inner = String(t.dropLast(2))
            let base = Self.closedClass[inner] ?? verb(inner) ?? inner
            if base.hasSuffix("ть") || base.hasSuffix("ти") || base.hasSuffix("чь") { return base + "ся" }
        }
        if let v = verb(t) { return v }
        if let a = adjective(t) { return a }
        return noun(t)
    }

    private func pick(_ candidates: [String], default fallback: String) -> String {
        candidates.first { vocabulary.contains($0) } ?? fallback
    }

    private func consonantFinal(_ s: String) -> Bool { s.last.map { !Self.vowels.contains($0) && $0 != "ь" && $0 != "й" } ?? false }

    // MARK: Verbs

    private func verb(_ t: String) -> String? {
        if t.hasSuffix("ть") || t.hasSuffix("чь") { return t }
        // -ти is an infinitive only for a known verb (власти, кости are nouns in -сть).
        if t.hasSuffix("ти"), vocabulary.contains(t) { return t }
        // Past tense.
        for (end, inf) in [("ала", "ать"), ("али", "ать"), ("ало", "ать"), ("ал", "ать"),
                           ("яла", "ять"), ("яли", "ять"), ("яло", "ять"), ("ял", "ять"),
                           ("ила", "ить"), ("или", "ить"), ("ило", "ить"), ("ил", "ить"),
                           ("ела", "еть"), ("ели", "еть"), ("ело", "еть"), ("ел", "еть"),
                           ("ула", "уть"), ("ули", "уть"), ("уло", "уть"), ("ул", "уть"),
                           ("ыла", "ыть"), ("ыли", "ыть"), ("ыло", "ыть"), ("ыл", "ыть")] where t.hasSuffix(end) {
            let stem = String(t.dropLast(end.count))
            // смеялся: the -ять class keeps a vowel before -л.
            guard stem.count >= 2, consonantFinal(stem) || inf == "ять" else { continue }
            if inf == "еть", stem.count < 3 { continue }  // дела, тела: nouns
            if end.count == 3, Self.velars.contains(stem.last!), inf == "ать", stem.count < 4 { continue }  // нога-ла? rare; keep conservative
            return pick([stem + inf], default: stem + inf)
        }
        // Present/future, vowel stems.
        for (end, inf) in [("аешь", "ать"), ("ает", "ать"), ("аем", "ать"), ("аете", "ать"), ("ают", "ать"), ("аю", "ать"),
                           ("яешь", "ять"), ("яет", "ять"), ("яем", "ять"), ("яете", "ять"), ("яют", "ять"), ("яю", "ять"),
                           ("уешь", "овать"), ("ует", "овать"), ("уем", "овать"), ("уете", "овать"), ("уют", "овать"),
                           ("еешь", "еть"), ("еет", "еть"), ("еем", "еть"), ("еете", "еть"), ("еют", "еть"), ("ею", "еть")]
            where t.hasSuffix(end) && t.count > end.count + 1 {
            let stem = String(t.dropLast(end.count))
            guard consonantFinal(stem) || stem.last == "й" else { continue }
            return stem + inf
        }
        // -ую is a verb (существую) only when the vocabulary says so; otherwise an adjective (дикую).
        if t.hasSuffix("ую"), t.count > 3, vocabulary.contains(String(t.dropLast(2)) + "овать") { return String(t.dropLast(2)) + "овать" }
        // Second conjugation: -ит family → -ить (hushing-stem -ать verbs are in the closed table).
        for end in ["ишь", "ите", "ит", "им", "ят", "ат"] where t.hasSuffix(end) && t.count > end.count + 2 {
            let stem = String(t.dropLast(end.count))
            guard consonantFinal(stem) else { continue }
            if end == "им" || end == "ат" {
                // also adjective (новым) and noun (солдат) endings: only a known verb
                guard vocabulary.contains(stem + "ить") else { continue }
            }
            if end == "ит", Self.nounsInIt.contains(t) { continue }
            return stem + "ить"
        }
        // Consonant-stem first conjugation (идёт, несёт, тянет): collapse the forms together.
        for end in ["ешь", "ете", "ет", "ем", "ут", "ют"] where t.hasSuffix(end) && t.count > end.count + 2 {
            let stem = String(t.dropLast(end.count))
            guard consonantFinal(stem) else { continue }
            if stem.hasSuffix("н") {
                // -нет: the -нуть class (тянет, прыгнет) unless it looks like a noun (момент, студент).
                if stem.hasSuffix("ен") || stem.hasSuffix("ан") || stem.hasSuffix("он") || end != "ет" && end != "ут" && end != "ешь" { continue }
                return pick([String(stem.dropLast()) + "нуть"], default: String(stem.dropLast()) + "нуть")
            }
            let candidates = [stem + "ать", stem + "ти", stem + "еть"]
            if end == "ем" || end == "ешь" || end == "ет" || end == "ете" {
                // -ем/-ет are also noun endings (обедом is -ом, but музеем, обед, совет): need a known verb,
                // except -ет after a consonant cluster typical of verbs (-ждет, -льет are closed).
                guard let known = candidates.first(where: { vocabulary.contains($0) }) else { continue }
                return known
            }
            return pick(candidates, default: stem + "ать")
        }
        // First person -у after a consonant (пишу, несу): only with a known verb.
        if t.hasSuffix("у"), t.count > 3, consonantFinal(String(t.dropLast())) {
            let stem = String(t.dropLast())
            if let v = [stem + "ать", stem + "ти"].first(where: { vocabulary.contains($0) }) { return v }
        }
        // -ю after a consonant: говорю → говорить; люблю → любить.
        if t.hasSuffix("ю"), t.count > 3 {
            let stem = String(t.dropLast())
            if consonantFinal(stem) {
                if stem.hasSuffix("л"), let p = stem.dropLast().last, ["б", "п", "в", "ф", "м"].contains(p) {
                    return String(stem.dropLast()) + "ить"
                }
                return stem + "ить"
            }
        }
        if let p = participle(t) { return p }
        // Short passive participles: напечатано → напечатать.
        for end in ["ано", "ана", "аны", "ан"] where t.hasSuffix(end) && t.count > end.count + 3 {
            let stem = String(t.dropLast(end.count))
            if stem.hasSuffix("ов") || stem.hasSuffix("ир") || stem.count >= 5 { return stem + "ать" }
        }
        return nil
    }

    /// Participles: an adjectival ending over a -ющ/-ащ/-вш/-анн stem.
    private func participle(_ t: String) -> String? {
        guard let stem = Self.adjectiveStem(t) else { return nil }
        for (end, inf) in [("ающ", "ать"), ("яющ", "ять"), ("ующ", "овать"), ("ащ", "ать"), ("ящ", "ить"),
                           ("авш", "ать"), ("явш", "ять"), ("ивш", "ить"), ("евш", "еть"), ("овавш", "овать"),
                           ("ованн", "овать"), ("анн", "ать")] where stem.hasSuffix(end) && stem.count > end.count + 2 {
            return String(stem.dropLast(end.count)) + inf
        }
        return nil
    }

    // MARK: Adjectives

    private static let adjectiveEndings = ["ими", "ыми", "его", "ого", "ему", "ому", "ая", "яя", "ое", "ее", "ые", "ие", "ую", "юю",
                                           "ых", "их", "ым", "им", "ой", "ей", "ий", "ый"]

    private static func adjectiveStem(_ t: String) -> String? {
        for end in adjectiveEndings where t.hasSuffix(end) && t.count > end.count + 2 {
            let stem = String(t.dropLast(end.count))
            guard let l = stem.last, !vowels.contains(l) else { continue }
            return stem
        }
        return nil
    }

    private func adjective(_ t: String) -> String? {
        // Endings that cannot be noun endings decide on their own.
        let decisive: [(String, String)] = [("ого", "ый"), ("его", "ий"), ("ому", "ый"), ("ему", "ий"), ("ыми", "ый"), ("ими", "ий"),
                                            ("ая", "ый"), ("яя", "ий"), ("ое", "ый"), ("ее", "ий"), ("ые", "ый"), ("ую", "ый"), ("юю", "ий"),
                                            ("ых", "ый"), ("ым", "ый")]
        for (end, restored) in decisive where t.hasSuffix(end) && t.count > end.count + 1 {
            let stem = String(t.dropLast(end.count))
            guard let l = stem.last, !Self.vowels.contains(l), l != "ь" else { continue }
            // -ая/-ое/-ые are also noun endings (стая, моё); the others aren't.
            if stem.count < 3, ["ая", "яя", "ое", "ее", "ые", "ую", "юю"].contains(end) { continue }
            let r = Self.velarsAndHushing.contains(l) ? "ий" : restored
            let oj = stem + "ой"
            if Self.ojAdjectives.contains(oj) || vocabulary.contains(oj) { return oj }
            return pick([stem + r, stem + "ый", stem + "ий"], default: stem + r)
        }
        // Feminine -ой on an adjective stem (государственной, славной): a consonant cluster ending
        // in н, or -нн/-ск/-ов/-ев; a lone vowel + н (женой, стеной) is a noun's instrumental.
        if t.hasSuffix("ой"), t.count > 4, !Self.ojAdjectives.contains(t) {
            let stem = String(t.dropLast(2))
            let chars = Array(stem)
            let clusterN = chars.count >= 2 && chars.last == "н" && !Self.vowels.contains(chars[chars.count - 2])
            if clusterN || stem.hasSuffix("ск") || stem.hasSuffix("ов") || stem.hasSuffix("ев") {
                let r = Self.velarsAndHushing.contains(chars.last!) ? "ий" : "ый"
                return pick([stem + r, stem + "ый", stem + "ий"], default: stem + r)
            }
        }
        if t.hasSuffix("ый") { return t }
        if t.hasSuffix("ий"), t.count > 4, let l = t.dropLast(2).last, !Self.vowels.contains(l), l != "н" || Self.softAdjectives.contains(t) || Self.velarsAndHushing.contains(l) {
            return t
        }
        if t.hasSuffix("ий"), t.count > 4, Self.velarsAndHushing.contains(t.dropLast(2).last!) || t.dropLast(2).hasSuffix("ск") { return t }
        if t.hasSuffix("ой"), Self.ojAdjectives.contains(t) || vocabulary.contains(t) { return t }
        for (end, restored) in [("ие", "ий"), ("их", "ий"), ("им", "ий")] where t.hasSuffix(end) && t.count > end.count + 3 {
            let stem = String(t.dropLast(end.count))
            guard let l = stem.last, !Self.vowels.contains(l), l != "ь" else { continue }
            let soft = stem + restored
            if vocabulary.contains(soft) || vocabulary.contains(stem + "ый") { return pick([soft, stem + "ый"], default: soft) }
            if Self.velarsAndHushing.contains(l) { return soft }
            if l == "н" {
                // последние (adjective) vs знание (noun in -ние): the noun unless the adjective is listed.
                if end == "ие" { return Self.softAdjectives.contains(soft) ? soft : nil }
                return soft
            }
        }
        return nil
    }

    // MARK: Nouns

    private func noun(_ t: String) -> String {
        func stem(_ n: Int) -> String { String(t.dropLast(n)) }
        func byLast(_ s: String, velar: String, other: String) -> String {
            s.last.map { Self.velarsAndHushing.contains($0) ? velar : other } ?? other
        }
        // -ия / -ие family.
        if t.hasSuffix("ии"), t.count > 4 {
            let s = stem(2)
            let abstract = s.hasSuffix("н") || s.hasSuffix("ыт") || s.hasSuffix("ят") || s.hasSuffix("вит")
            return pick([s + "ие", s + "ия"], default: abstract ? s + "ие" : s + "ия")
        }
        if t.hasSuffix("ию"), t.count > 4 { return pick([stem(2) + "ия", stem(2) + "ие"], default: stem(2) + "ия") }
        if t.hasSuffix("ием") || t.hasSuffix("иям"), t.count > 5 { return stem(3) + "ие" }
        if t.hasSuffix("иями"), t.count > 5 { return pick([stem(4) + "ие", stem(4) + "ия"], default: stem(4) + "ие") }
        if t.hasSuffix("иях"), t.count > 4 { return pick([stem(3) + "ие", stem(3) + "ия"], default: stem(3) + "ие") }
        if t.hasSuffix("ие") || t.hasSuffix("ье") || t.hasSuffix("ия") || t.hasSuffix("ья") { return t }
        // -ство is never feminine: государства, пространству, веществом → -ство.
        for end in ["ства", "ству", "ством", "стве", "ств"] where t.hasSuffix(end) && t.count > end.count + 2 {
            return String(t.dropLast(end.count)) + "ство"
        }
        // Diminutive fleeting vowel: котёнка → котёнок.
        for end in ["енка", "енку", "енком", "енке"] where t.hasSuffix(end) && t.count > end.count + 2 {
            return String(t.dropLast(end.count)) + "енок"
        }
        if t.hasSuffix("ью"), t.count > 3 { return stem(2) + "ь" }
        if t.hasSuffix("ами"), t.count > 4 {
            let s = stem(3)
            return pick([s, s + "а", s + "о"], default: byLast(s, velar: s + "а", other: s))
        }
        if t.hasSuffix("ями"), t.count > 4 { return pick([stem(3) + "я", stem(3) + "ь"], default: stem(3) + "я") }
        if t.hasSuffix("ах") || t.hasSuffix("ам"), t.count > 3 {
            let s = stem(2)
            return pick([s, s + "а", s + "о"], default: byLast(s, velar: s + "а", other: s))
        }
        if t.hasSuffix("ях") || t.hasSuffix("ям"), t.count > 3 { return pick([stem(2) + "я", stem(2) + "ь"], default: stem(2) + "я") }
        if t.hasSuffix("ов"), t.count > 3 { return stem(2) }
        if t.hasSuffix("ев"), t.count > 3 {
            let s = stem(2)
            if s.last == "ц" { return s }
            return s.last.map { Self.vowels.contains($0) ? s + "й" : s + "ь" } ?? s
        }
        if t.hasSuffix("ей"), t.count > 3 {
            let s = stem(2)
            let soft = s.last.map { Self.vowels.contains($0) ? s + "я" : s + "ь" } ?? s
            return pick([s + "ь", s + "я", s + "а", s], default: soft)
        }
        if t.hasSuffix("ой") || t.hasSuffix("ою"), t.count > 3 { return pick([stem(2) + "а", stem(2)], default: stem(2) + "а") }
        if t.hasSuffix("ею"), t.count > 3 { return stem(2) + "я" }
        if t.hasSuffix("ом"), t.count > 3 { return pick([stem(2), stem(2) + "о"], default: stem(2)) }
        if t.hasSuffix("ем"), t.count > 3 {
            let s = stem(2)
            let soft = s.last.map { Self.vowels.contains($0) ? s + "й" : s + "ь" } ?? s
            return pick([s + "ь", s + "й", s + "е"], default: soft)
        }
        if t.hasSuffix("у"), t.count > 3 {
            let s = stem(1)
            return pick([s, s + "а", s + "о"], default: byLast(s, velar: s + "а", other: s))
        }
        if t.hasSuffix("ю"), t.count > 3 { return pick([stem(1) + "я", stem(1) + "ь"], default: stem(1) + "я") }
        if t.hasSuffix("е"), t.count > 3 {
            let s = stem(1)
            if let l = s.last, Self.vowels.contains(l) || l == "ь" { return t }
            return pick([s, s + "а", s + "о", t], default: byLast(s, velar: s + "а", other: s))
        }
        if t.hasSuffix("ы"), t.count > 3 { return pick([stem(1), stem(1) + "а"], default: stem(1)) }
        if t.hasSuffix("и"), t.count > 3 {
            let s = stem(1)
            if let l = s.last {
                if s.hasSuffix("ик") { return pick([s, s + "а"], default: s) }
                if Self.velars.contains(l) { return pick([s + "а", s], default: s + "а") }
                if Self.hushing.contains(l) { return pick([s, s + "а", s + "ь"], default: s) }
                if Self.vowels.contains(l) { return pick([s + "й", s + "я"], default: s + "й") }
                return pick([s + "ь", s, s + "а", s + "я"], default: s + "ь")
            }
        }
        if t.hasSuffix("а") || t.hasSuffix("я"), t.count > 3 {
            let s = stem(1)
            if vocabulary.contains(s) { return s }
            if t.hasSuffix("я"), let l = s.last, Self.vowels.contains(l), vocabulary.contains(s + "й") { return s + "й" }
            return t
        }
        return t
    }
}
