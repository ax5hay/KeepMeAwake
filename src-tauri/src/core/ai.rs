//! What is asked of Claude, and how its answers are read. Pure: this builds
//! request bodies and reads response bodies; the key, the network and the
//! switches live in `app::ai`.
//!
//! Claude is a choice here. Nothing in this module runs unless someone has
//! given the app a key, switched Claude on, left the feature's own switch
//! on, and then asked: there is no request the app makes on its own.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The model asked: the smallest and cheapest there is. Everything asked of
/// it here is a sentence or two from a few lines, which is its kind of work.
pub const MODEL: &str = "claude-haiku-5-5";

/// The Messages API version sent with every request.
pub const API_VERSION: &str = "2026-10-21";

/// The most a request may be answered with, per kind of answer. Thinking is
/// switched off, so this is all the output there can be, and so all it can
/// cost: a line, a few sentences, or one small JSON object.
pub const MAX_TOKENS_LINE: u32 = 120;
/// See [`MAX_TOKENS_LINE`].
pub const MAX_TOKENS_WORDS: u32 = 300;
/// See [`MAX_TOKENS_LINE`].
pub const MAX_TOKENS_PLAN: u32 = 200;

/// The longest digest or sentence sent. Longer is refused, never cut: a
/// month of days is about three thousand characters, so more means
/// something is wrong, and half a month would be read as a whole one. It is
/// also the ceiling on what one request can cost to send.
pub const MAX_INPUT_CHARS: usize = 6_000;

/// The longest question or sentence someone may type.
pub const MAX_QUESTION_CHARS: usize = 300;

/// Why Claude gave no answer, in terms a person can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    /// No API key has been given to the app.
    NoKey,
    /// Claude, or this feature, is switched off.
    Off,
    /// Anthropic turned the key down.
    BadKey,
    /// Rate limited or overloaded; trying again later may work.
    Busy,
    /// The request was declined by the model.
    Declined,
    /// What was to be sent is longer than [`MAX_INPUT_CHARS`].
    TooLong,
    /// Nothing usable came back.
    Empty,
    /// The network, or Anthropic, failed in some other way.
    Failed(String),
}

impl AiError {
    /// The sentence the window shows.
    pub fn sentence(&self) -> String {
        match self {
            Self::NoKey => "Claude is locked: no API key has been added.".to_owned(),
            Self::Off => "Claude is switched off.".to_owned(),
            Self::BadKey => "Anthropic did not accept that API key.".to_owned(),
            Self::Busy => "Claude is busy just now. Try again in a minute.".to_owned(),
            Self::Declined => "Claude declined to answer that.".to_owned(),
            Self::TooLong => "That is too long to send.".to_owned(),
            Self::Empty => "Claude sent nothing back.".to_owned(),
            Self::Failed(why) => format!("Claude could not be reached: {why}"),
        }
    }

    /// The error for an HTTP status that is not a success.
    pub fn from_status(status: u16) -> Self {
        match status {
            401 | 403 => Self::BadKey,
            429 | 529 => Self::Busy,
            other => Self::Failed(format!("HTTP {other}")),
        }
    }
}

/// How the week is to be put into words.
const WEEK_SYSTEM: &str = "You read one week from a personal almanac kept by a small desktop app. \
Each line is a day: its hours at the keyboard, the first and last touch, the time outside \
working hours, and the night that led into it (the quiet between the last touch of one day and \
the first of the next). Write what this week was like in two or three plain sentences, for the \
person whose week it is, addressed as you. Say what stands out: the long day, the short night, \
the day off, hours that crept later. Use the times and figures given and no others. The app \
knows when the keys went quiet, not what the person did, so say night or quiet, never that they \
slept, and do not give advice or judge. No lists, no headings, no preamble.";

/// How a sentence is to be turned into a plan.
const PLAN_SYSTEM: &str = "You turn one sentence into a plan for an app that keeps a laptop awake. \
The app can do four things: keep it awake for a length of time, keep it awake until a time of \
day, keep it awake until told otherwise, or let it sleep. Read the sentence and choose one. \
For a length of time, give whole minutes. For a time of day, give it as 24-hour HH:MM in the \
person's local time; the current local time is given so that words like tonight, noon or in \
the morning can be resolved. If the sentence asks for something else or you cannot tell, choose \
unclear. In `say`, give the plan back in a few plain words, as the app would say it, such as \
'Awake for 2 hours' or 'Awake until 6:30 pm'; for unclear, say in a few words what was not \
understood.";

/// How one day is to be put into a line.
const DAY_SYSTEM: &str = "You read one day from a personal almanac kept by a small desktop app: \
its hours at the keyboard, the first and last touch, the time outside working hours, and the \
night that led into it. Say what the day was in one plain sentence of at most twenty words, to \
the person whose day it is, addressed as you. Use the times and figures given and no others. \
Say night or quiet, never that they slept. No advice, no judgement, no preamble.";

/// How a question about the almanac is to be answered.
const ASK_SYSTEM: &str = "You answer one question about a personal almanac kept by a small desktop \
app. You are given some days, one line each: hours at the keyboard, first and last touch, time \
outside working hours, the night that led into the day, and whether it was leave. Answer from \
those lines only, in one or two plain sentences, addressed as you, working out any totals or \
averages yourself. If the lines do not hold the answer, say so in a few words. Say night or \
quiet, never that they slept. No advice, no preamble.";

/// The body every request shares: the smallest model, thinking off and
/// effort low so that no token is spent on anything but the answer, and a
/// hard cap on the answer's length.
fn body(system: &str, content: &str, max_tokens: u32) -> Value {
    json!({
        "model": MODEL,
        "max_tokens": max_tokens,
        "thinking": { "type": "disabled" },
        "output_config": { "effort": "low" },
        "system": system,
        "messages": [{ "role": "user", "content": content }],
    })
}

/// The request that puts a week into words. `digest` is the week as the
/// window lays it out, one day a line.
pub fn week_request(digest: &str) -> Result<Value, AiError> {
    fits(digest, MAX_INPUT_CHARS)?;
    Ok(body(WEEK_SYSTEM, digest, MAX_TOKENS_WORDS))
}

/// The request that puts one day into a line. `digest` is that day's line.
pub fn day_request(digest: &str) -> Result<Value, AiError> {
    fits(digest, MAX_INPUT_CHARS)?;
    Ok(body(DAY_SYSTEM, digest, MAX_TOKENS_LINE))
}

/// The request that answers `question` from `digest`, the days in view.
pub fn ask_request(question: &str, digest: &str) -> Result<Value, AiError> {
    fits(question, MAX_QUESTION_CHARS)?;
    fits(digest, MAX_INPUT_CHARS)?;
    Ok(body(
        ASK_SYSTEM,
        &format!("{digest}\n\nQuestion: {question}"),
        MAX_TOKENS_WORDS,
    ))
}

/// The request that turns `sentence` into a plan. `now_local` is the local
/// date and time as the window has it, `Tue 10 Nov 2026, 18:42`.
pub fn plan_request(sentence: &str, now_local: &str) -> Result<Value, AiError> {
    fits(sentence, MAX_QUESTION_CHARS)?;
    let mut body = body(
        PLAN_SYSTEM,
        &format!("It is now {now_local}.\n\n{sentence}"),
        MAX_TOKENS_PLAN,
    );
    body["output_config"]["format"] = json!({
        "type": "json_schema",
        "schema": {
            "type": "object",
            "properties": {
                "kind": {
                    "type": "string",
                    "enum": ["for", "until", "indefinite", "off", "unclear"],
                },
                "minutes": { "type": "integer" },
                "time": { "type": "string" },
                "say": { "type": "string" },
            },
            "required": ["kind", "minutes", "time", "say"],
            "additionalProperties": false,
        },
    });
    Ok(body)
}

fn fits(text: &str, most: usize) -> Result<(), AiError> {
    if text.trim().is_empty() {
        return Err(AiError::Empty);
    }
    if text.chars().count() > most {
        return Err(AiError::TooLong);
    }
    Ok(())
}

/// The words of a response: its text blocks, joined. A response the model
/// declined, or one with no text in it, is an error, checked before the
/// content is read.
pub fn text_of(response: &Value) -> Result<String, AiError> {
    if response["stop_reason"] == "refusal" {
        return Err(AiError::Declined);
    }
    let text: Vec<&str> = response["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|block| block["type"] == "text")
        .filter_map(|block| block["text"].as_str())
        .collect();
    let text = text.join("").trim().to_owned();
    if text.is_empty() {
        Err(AiError::Empty)
    } else {
        Ok(text)
    }
}

/// What kind of plan a sentence was understood as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Understanding {
    /// Awake for `minutes`.
    For,
    /// Awake until `time`.
    Until,
    /// Awake until told otherwise.
    Indefinite,
    /// Let it sleep.
    Off,
    /// Not something the app can do, or not clear enough to act on.
    Unclear,
}

/// A sentence, understood. The window shows `say` and asks before it acts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Understood {
    /// What kind of plan.
    pub kind: Understanding,
    /// For `for`: how long, in minutes. Zero otherwise.
    pub minutes: u32,
    /// For `until`: the local time of day, `18:30`. Empty otherwise.
    pub time: String,
    /// The plan in a few plain words, or what was not understood.
    pub say: String,
}

/// A week is the longest the app will keep a laptop awake for on a timer.
const MAX_MINUTES: i64 = 7 * 24 * 60;

/// Reads the plan out of a response. Anything that does not hold together
/// (a length of no minutes, a time that is not one) reads as unclear, so
/// the window never acts on half an answer.
pub fn understood(response: &Value) -> Result<Understood, AiError> {
    let text = text_of(response)?;
    let raw: Value = serde_json::from_str(&text).map_err(|_| AiError::Empty)?;
    let kind: Understanding =
        serde_json::from_value(raw["kind"].clone()).map_err(|_| AiError::Empty)?;
    let minutes = raw["minutes"].as_i64().unwrap_or(0);
    let time = raw["time"].as_str().unwrap_or("").trim().to_owned();
    let say = raw["say"].as_str().unwrap_or("").trim().to_owned();
    let holds = match kind {
        Understanding::For => (1..=MAX_MINUTES).contains(&minutes),
        Understanding::Until => is_time(&time),
        _ => true,
    };
    let kind = if holds { kind } else { Understanding::Unclear };
    Ok(Understood {
        kind,
        minutes: if kind == Understanding::For {
            u32::try_from(minutes).unwrap_or(0)
        } else {
            0
        },
        time: if kind == Understanding::Until {
            time
        } else {
            String::new()
        },
        say,
    })
}

/// `HH:MM`, 24-hour.
fn is_time(time: &str) -> bool {
    let Some((hours, minutes)) = time.split_once(':') else {
        return false;
    };
    let in_range = |part: &str, max: u8| {
        (1..=2).contains(&part.len()) && part.parse::<u8>().is_ok_and(|n| n <= max)
    };
    in_range(hours, 23) && minutes.len() == 2 && in_range(minutes, 59)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(text: &str) -> Value {
        json!({
            "stop_reason": "end_turn",
            "content": [
                { "type": "thinking", "thinking": "" },
                { "type": "text", "text": text },
            ],
        })
    }

    #[test]
    fn every_request_is_as_cheap_as_a_request_can_be() {
        let bodies = [
            (
                week_request("Mon 9 Nov: 10h 56m").unwrap(),
                MAX_TOKENS_WORDS,
            ),
            (day_request("Mon 9 Nov: 10h 56m").unwrap(), MAX_TOKENS_LINE),
            (
                ask_request("How long?", "Mon 9 Nov: 10h 56m").unwrap(),
                MAX_TOKENS_WORDS,
            ),
            (plan_request("two hours", "now").unwrap(), MAX_TOKENS_PLAN),
        ];
        for (body, cap) in &bodies {
            assert_eq!(body["model"], "claude-haiku-5-5", "the smallest model");
            assert_eq!(
                body["thinking"]["type"], "disabled",
                "nothing spent thinking"
            );
            assert_eq!(body["output_config"]["effort"], "low");
            assert_eq!(body["max_tokens"], *cap);
            assert!(*cap <= 300, "an answer is a few sentences at most");
            assert!(body.get("temperature").is_none(), "rejected by this model");
            assert!(
                body.get("fallbacks").is_none(),
                "no second model, no second bill"
            );
            assert!(body.get("tools").is_none());
        }
    }

    #[test]
    fn the_week_request_carries_the_digest_and_how_to_speak_of_it() {
        let body = week_request("Mon 9 Nov: 10h 56m").unwrap();
        assert_eq!(body["messages"][0]["content"], "Mon 9 Nov: 10h 56m");
        let system = body["system"].as_str().unwrap();
        assert!(
            system.contains("never that they"),
            "it says night, not sleep"
        );
    }

    #[test]
    fn the_plan_request_asks_for_json_of_one_shape_and_gives_the_time() {
        let body = plan_request("two hours please", "Tue 10 Nov 2026, 18:42").unwrap();
        let format = &body["output_config"]["format"];
        assert_eq!(format["type"], "json_schema");
        assert_eq!(format["schema"]["additionalProperties"], false);
        assert_eq!(
            format["schema"]["required"],
            json!(["kind", "minutes", "time", "say"])
        );
        let content = body["messages"][0]["content"].as_str().unwrap();
        assert!(content.starts_with("It is now Tue 10 Nov 2026, 18:42."));
        assert!(content.ends_with("two hours please"));
    }

    #[test]
    fn nothing_and_too_much_are_refused_before_anything_is_sent() {
        assert_eq!(week_request("  "), Err(AiError::Empty));
        let long = "x".repeat(MAX_INPUT_CHARS + 1);
        assert_eq!(week_request(&long), Err(AiError::TooLong));
        assert_eq!(day_request(&long), Err(AiError::TooLong));
        assert!(week_request(&"x".repeat(MAX_INPUT_CHARS)).is_ok());
        let wordy = "x".repeat(MAX_QUESTION_CHARS + 1);
        assert_eq!(plan_request(&wordy, "now"), Err(AiError::TooLong));
        assert_eq!(ask_request(&wordy, "Mon: 8h"), Err(AiError::TooLong));
        assert_eq!(ask_request("How long?", &long), Err(AiError::TooLong));
        assert_eq!(ask_request(" ", "Mon: 8h"), Err(AiError::Empty));
    }

    #[test]
    fn a_question_is_sent_under_the_days_it_is_about() {
        let body = ask_request("How many hours?", "Mon 9 Nov: 8h\nTue 10 Nov: 7h").unwrap();
        assert_eq!(
            body["messages"][0]["content"],
            "Mon 9 Nov: 8h\nTue 10 Nov: 7h\n\nQuestion: How many hours?"
        );
        let system = body["system"].as_str().unwrap();
        assert!(system.contains("those lines only"));
    }

    #[test]
    fn the_words_are_the_text_blocks_and_a_refusal_is_not_read() {
        assert_eq!(
            text_of(&reply(" A long Monday. ")).unwrap(),
            "A long Monday."
        );
        let declined = json!({
            "stop_reason": "refusal",
            "content": [{ "type": "text", "text": "partial" }],
        });
        assert_eq!(text_of(&declined), Err(AiError::Declined));
        assert_eq!(text_of(&json!({ "content": [] })), Err(AiError::Empty));
        assert_eq!(text_of(&json!({})), Err(AiError::Empty));
    }

    #[test]
    fn a_plan_is_read_out_of_the_json() {
        let two_hours =
            reply(r#"{"kind":"for","minutes":120,"time":"","say":"Awake for 2 hours"}"#);
        assert_eq!(
            understood(&two_hours).unwrap(),
            Understood {
                kind: Understanding::For,
                minutes: 120,
                time: String::new(),
                say: "Awake for 2 hours".to_owned(),
            }
        );
        let evening =
            reply(r#"{"kind":"until","minutes":0,"time":"18:30","say":"Awake until 6:30 pm"}"#);
        let evening = understood(&evening).unwrap();
        assert_eq!(evening.kind, Understanding::Until);
        assert_eq!(evening.time, "18:30");
    }

    #[test]
    fn half_an_answer_reads_as_unclear() {
        for text in [
            r#"{"kind":"for","minutes":0,"time":"","say":"Awake"}"#,
            r#"{"kind":"for","minutes":99999999,"time":"","say":"Awake forever"}"#,
            r#"{"kind":"until","minutes":0,"time":"half six","say":"Awake until half six"}"#,
            r#"{"kind":"until","minutes":0,"time":"25:00","say":"Awake"}"#,
        ] {
            let read = understood(&reply(text)).unwrap();
            assert_eq!(read.kind, Understanding::Unclear, "{text}");
            assert_eq!(read.minutes, 0);
            assert_eq!(read.time, "");
        }
        assert_eq!(understood(&reply("not json")), Err(AiError::Empty));
        assert_eq!(
            understood(&reply(r#"{"kind":"dance","minutes":0,"time":"","say":""}"#)),
            Err(AiError::Empty)
        );
    }

    #[test]
    fn a_status_becomes_something_a_person_can_act_on() {
        assert_eq!(AiError::from_status(401), AiError::BadKey);
        assert_eq!(AiError::from_status(429), AiError::Busy);
        assert_eq!(AiError::from_status(529), AiError::Busy);
        assert_eq!(
            AiError::from_status(500),
            AiError::Failed("HTTP 500".to_owned())
        );
        for error in [AiError::NoKey, AiError::Off, AiError::BadKey, AiError::Busy] {
            assert!(!error.sentence().contains('!'));
        }
    }

    #[test]
    fn understood_crosses_to_the_window_in_snake_case() {
        let read = Understood {
            kind: Understanding::Indefinite,
            minutes: 0,
            time: String::new(),
            say: "Awake until you say otherwise".to_owned(),
        };
        let json = serde_json::to_value(&read).unwrap();
        assert_eq!(json["kind"], "indefinite");
        assert_eq!(json["minutes"], 0);
    }
}
