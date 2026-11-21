//! The four ways in, as the pages read them: the titles, the steps and the
//! terms each applicant accepts before signing. Text only — `src/apply.rs` is
//! the funnel that draws it.

pub struct Kind {
    pub what: &'static str,
    pub doc_title: &'static str,
    pub title: &'static str,
    pub sub: &'static str,
    /// the second step: its name and its line
    pub thing: (&'static str, &'static str),
    /// the third step: its name, its line, and what the applicant accepts
    pub terms_step: (&'static str, &'static str),
    pub terms: &'static [(&'static str, &'static str)],
    pub accept: &'static str,
    /// what a waiting application is told
    pub wait: &'static str,
}

const KINDS: &[Kind] = &[
    Kind {
        what: "land",
        doc_title: "Cyberia — add your land",
        title: "Add your land",
        sub: "land that is not on the map yet · the valley checks the paper, draws it, and it sells like every flat",
        thing: ("THE LAND", "where it is, how large, the paper"),
        terms_step: ("TERMS", "how land gets onto the map"),
        terms: &[
            ("THE PAPER", "the valley checks the right you name against its certificate before anything is drawn"),
            ("THE BOUNDARY", "the land is drawn on the map from a survey and cut into flats"),
            ("THEN THE OFFER", "the flats are bound to your wallet; you or your agent put them up at SELL, and the valley approves each offer"),
            ("NOTHING IS PAID HERE", "an application costs nothing and binds nobody"),
        ],
        accept: "I HOLD THIS LAND OR SPEAK FOR ITS HOLDER",
        wait: "the valley reads it and writes to you about the paper",
    },
    Kind {
        what: "agent",
        doc_title: "Cyberia — become an agent",
        title: "Become an agent",
        sub: "sell and sublease flats for their holders · accredited by the valley · every offer under a signed mandate",
        thing: ("YOU", "where you sell, in which languages"),
        terms_step: ("THE CODE", "what an agent of the valley keeps to"),
        terms: &[
            ("A MANDATE, ALWAYS", "you offer a flat only under a mandate its holder signed for your address: the flat, sale or sublease, the commission, the last day"),
            ("THE COMMISSION", "is the number in the mandate, agreed between you and the holder"),
            ("THE VALLEY APPROVES", "every offer you sign waits for the valley's approval before the map shows it"),
            ("THE RIGHT AS IT IS", "a flat is a leasehold for the years it has left; you say it so, and the flat's page is your source"),
            ("ONE SIGNATURE ENDS IT", "the holder, you or the valley revoke a mandate with one signature, and the offers under it come down"),
            ("THE BADGE IS THE VALLEY'S", "accreditation shows beside your offers and in the directory holders pick from; the valley gives it and can take it back"),
            ("WHAT IS PUBLISHED", "accredited, the directory carries your wallet address, the name you chose, your territory and your languages, open to everyone — a holder names that address in the mandate. your telegram is not published: it stays with the valley"),
        ],
        accept: "I ACCEPT THE AGENT'S CODE",
        wait: "the valley reads it and writes to you; accredited, this page becomes your desk",
    },
    Kind {
        what: "franchise",
        doc_title: "Cyberia — buy a franchise",
        title: "Buy a franchise",
        sub: "found cities of Cyberia in your country or territory · one application per territory",
        thing: ("THE TERRITORY", "a state, a region or a district — and who stands behind it"),
        terms_step: ("TERMS", "what a franchise is and how it is agreed"),
        terms: &[
            ("WHAT IT IS", "the right to found cities of Cyberia in one territory, under its name, its map and its rules"),
            ("ONE TERRITORY", "a state, a region or a district, as the globe draws it"),
            ("AGREED PER TERRITORY", "the fee, the share and the term are agreed for each territory after the valley reads the application; nothing is paid here"),
            ("WHAT THE VALLEY READS", "who stands behind it, the land or the capital in hand, the people on the ground"),
            ("WHERE IT IS OPEN", "today cities are founded in Indonesia; an application for another territory opens the talk about it"),
        ],
        accept: "I ASK FOR THIS FRANCHISE AND WILL AGREE ITS TERMS WITH THE VALLEY",
        wait: "the valley reads it and writes to you about the territory's terms",
    },
    Kind {
        what: "city",
        doc_title: "Cyberia — found a city",
        title: "Found a city",
        sub: "a place on the globe and a name · the valley answers every application",
        thing: ("THE CITY", "its name, its place, what stands behind it"),
        terms_step: ("TERMS", "what founding asks for"),
        terms: &[
            ("A PLACE AND A NAME", "the point you marked on the globe and the name the city will carry"),
            ("UNDER A FRANCHISE", "a city stands under the franchise of its territory; where none is given yet, its founder asks for it or joins the one who holds it"),
            ("LAND IN HAND", "a city starts from land with a paper — say what you hold or are about to"),
            ("NOTHING IS PAID HERE", "an application costs nothing and binds nobody"),
        ],
        accept: "I WANT TO FOUND THIS CITY",
        wait: "the valley reads it and writes to you",
    },
];

pub fn kind_of(what: &str) -> &'static Kind {
    KINDS.iter().find(|k| k.what == what).unwrap_or(&KINDS[0])
}
